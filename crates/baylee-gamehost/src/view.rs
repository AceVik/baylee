//! Building per-seat views (CR 400.2) from engine state.
//!
//! The wire types live in [`baylee_view`] so that clients do not have to link
//! the rules kernel. This module is the only place that translates engine
//! state into them, and it is where the hidden-information rules are enforced:
//! a seat sees public zones in full, its own hand, and only counts for hidden
//! zones belonging to anyone else.
//!
//! Characteristics are taken from the engine's **projected** values, not from
//! the printed card. A client cannot run the layer system, so an anthem, a
//! clone, or an animated land has to arrive already resolved.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::turn::{Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::ZoneLocation;
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, CounterEntry, CounterKind, GameStatic,
    HandObject, ObjectStatus, Phase, PlayerView, PublicObject, SeatView, Step, TargetRef,
};

pub use baylee_view as wire;

/// Translates the engine's phase into the wire enum.
const fn phase(p: EnginePhase) -> Phase {
    match p {
        EnginePhase::Beginning => Phase::Beginning,
        EnginePhase::FirstMain => Phase::FirstMain,
        EnginePhase::Combat => Phase::Combat,
        EnginePhase::SecondMain => Phase::SecondMain,
        EnginePhase::Ending => Phase::Ending,
    }
}

/// Translates the engine's step into the wire enum.
const fn step(s: EngineStep) -> Step {
    match s {
        EngineStep::Untap => Step::Untap,
        EngineStep::Upkeep => Step::Upkeep,
        EngineStep::Draw => Step::Draw,
        EngineStep::Main => Step::Main,
        EngineStep::CombatBegin => Step::CombatBegin,
        EngineStep::DeclareAttackers => Step::DeclareAttackers,
        EngineStep::DeclareBlockers => Step::DeclareBlockers,
        EngineStep::CombatDamageFirst => Step::CombatDamageFirst,
        EngineStep::CombatDamage => Step::CombatDamage,
        EngineStep::CombatEnd => Step::CombatEnd,
        EngineStep::End => Step::End,
        EngineStep::Cleanup => Step::Cleanup,
    }
}

/// Translates a counter kind into the wire enum.
const fn counter(kind: baylee_cards_dsl::CounterKind) -> CounterKind {
    use baylee_cards_dsl::CounterKind as K;
    match kind {
        K::P1P1 => CounterKind::PlusOnePlusOne,
        K::M1M1 => CounterKind::MinusOneMinusOne,
        K::Loyalty => CounterKind::Loyalty,
        K::Lore => CounterKind::Lore,
        K::Time => CounterKind::Time,
        K::Charge => CounterKind::Charge,
        K::Poison => CounterKind::Poison,
        K::Energy => CounterKind::Energy,
        K::Rad => CounterKind::Rad,
        K::Lifelink => CounterKind::Lifelink,
        K::Level => CounterKind::Level,
        K::Custom(id) => CounterKind::Custom(id as u32),
    }
}

/// Whether `seat` is entitled to know what card backs `obj`.
///
/// Face-down permanents are the one case where two seats looking at the same
/// battlefield legitimately see different things (CR 707.2): the controller
/// knows what they played, everyone else sees a blank. Returning `None` for
/// the card identity — rather than sending it and trusting the client to hide
/// it — is what makes the leak unrepresentable.
fn may_know_card(obj: &GameObject, seat: PlayerId) -> bool {
    !obj.status
        .contains(baylee_engine::object::Status::FACE_DOWN)
        || obj.controller == seat
}

/// The public name of an object as `seat` may know it.
fn public_name(state: &GameState, obj: &GameObject, seat: PlayerId) -> String {
    if may_know_card(obj, seat) {
        state.names.get(obj.characteristics().name).to_string()
    } else {
        "Face-down".to_string()
    }
}

/// Projects one object into its public form for `seat`.
fn public_object(state: &GameState, id: ObjectId, seat: PlayerId) -> Option<PublicObject> {
    let obj = state.object(id)?;
    let chars = obj.characteristics();
    let known = may_know_card(obj, seat);
    Some(PublicObject {
        id,
        card: obj.card.filter(|_| known).map(|c| CardIdentity {
            index: c.index,
            print: c.print,
            face: obj.face_index,
        }),
        name: public_name(state, obj, seat),
        controller: obj.controller,
        owner: obj.owner,
        status: ObjectStatus::from_bits(obj.status.bits()),
        types: chars.types,
        supertypes: chars.supertypes,
        subtypes: chars.subtypes,
        // The engine holds the definition; the client needs the number, and
        // this crate is the one that can see both.
        token: obj.token.map(baylee_cards::tokens::token_id),
        colors: chars.colors,
        keywords: chars.keywords.bits(),
        power: chars.power,
        toughness: chars.toughness,
        loyalty: chars.loyalty,
        mana_value: chars.mana_cost.cmc(),
        damage: obj.damage,
        counters: obj
            .counters
            .iter()
            .map(|(kind, count)| CounterEntry {
                kind: counter(kind),
                count,
            })
            .collect(),
        attached_to: obj.attached_to,
        targets: obj.targets.iter().map(|t| TargetRef::Object(*t)).collect(),
        stack_item: stack_item(obj),
        summoning_sick: obj.kind == ObjectKind::Permanent
            && baylee_engine::combat::summoning_sick(state, obj),
    })
}

/// What a stack object is, for objects that are on the stack.
///
/// An ability on the stack is its own object with no card of its own, so
/// without this a client can only draw an anonymous entry — it knows a
/// trigger is resolving, but not whose, and not which of that permanent's
/// abilities it is. The engine already tracks exactly that in
/// `AbilityLoc`; this is where it reaches the client.
fn stack_item(obj: &GameObject) -> Option<baylee_view::StackItem> {
    use baylee_view::StackItem;
    match obj.kind {
        ObjectKind::Spell => Some(StackItem::Spell),
        ObjectKind::AbilityOnStack => obj.ability.map(|loc| StackItem::Ability {
            source: loc.source,
            ability: baylee_core::ids::AbilityRef::new(loc.card, loc.index),
        }),
        _ => None,
    }
}

/// Collects a public zone into view objects.
fn zone(state: &GameState, loc: ZoneLocation, seat: PlayerId) -> Vec<PublicObject> {
    state
        .zones
        .list(loc)
        .iter()
        .filter_map(|id| public_object(state, *id, seat))
        .collect()
}

/// Collects one zone per seat, indexed by seat order.
fn per_seat_zone(
    state: &GameState,
    loc: fn(PlayerId) -> ZoneLocation,
    seat: PlayerId,
) -> Vec<Vec<PublicObject>> {
    state
        .players
        .iter()
        .map(|p| zone(state, loc(p.id), seat))
        .collect()
}

/// The floating mana of one seat, for the view.
fn mana_pool(pool: &baylee_core::mana::ManaPool) -> baylee_view::ManaPoolView {
    use baylee_core::mana::ManaColor;
    baylee_view::ManaPoolView {
        white: pool.available(ManaColor::White),
        blue: pool.available(ManaColor::Blue),
        black: pool.available(ManaColor::Black),
        red: pool.available(ManaColor::Red),
        green: pool.available(ManaColor::Green),
        colorless: pool.available(ManaColor::Colorless),
        restricted: pool.restricted().iter().map(|r| r.amount).sum(),
    }
}

/// Builds the hidden-information-filtered view of `state` for `seat`.
///
/// `priority` is the seat that currently holds priority, which the engine
/// tracks in its pending choice rather than in the state itself.
#[must_use]
pub fn player_view(
    state: &GameState,
    seat: PlayerId,
    priority: Option<PlayerId>,
    seq: u64,
) -> PlayerView {
    let hand = state
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter_map(|id| {
            let obj = state.object(*id)?;
            let card = obj.card?;
            let chars = obj.characteristics();
            Some(HandObject {
                id: *id,
                card: CardIdentity {
                    index: card.index,
                    print: card.print,
                    face: obj.face_index,
                },
                name: state.names.get(chars.name).to_string(),
                mana_value: chars.mana_cost.cmc(),
                colors: chars.colors,
                types: chars.types,
            })
        })
        .collect();

    PlayerView {
        seq,
        seat,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        priority,
        monarch: state.monarch,
        seats: state
            .players
            .iter()
            .map(|p| SeatView {
                player: p.id,
                life: p.life,
                poison: p.poison,
                energy: p.energy,
                hand_count: state.zones.list(ZoneLocation::Hand(p.id)).len() as u32,
                library_count: state.zones.list(ZoneLocation::Library(p.id)).len() as u32,
                graveyard_count: state.zones.list(ZoneLocation::Graveyard(p.id)).len() as u32,
                has_lost: p.has_lost,
                mana_pool: mana_pool(&p.mana_pool),
                commander_casts: state.commander_casts.clone(),
            })
            .collect(),
        hand,
        battlefield: zone(state, ZoneLocation::Battlefield, seat),
        stack: zone(state, ZoneLocation::Stack, seat),
        graveyards: per_seat_zone(state, ZoneLocation::Graveyard, seat),
        exile: per_seat_zone(state, ZoneLocation::Exile, seat),
        command: per_seat_zone(state, ZoneLocation::Command, seat),
        combat: CombatView {
            attackers: state
                .combat
                .attackers
                .iter()
                .map(|a| AttackerView {
                    creature: a.creature,
                    defending: a.defending,
                })
                .collect(),
            blockers: state
                .combat
                .blockers
                .iter()
                .map(|b| BlockerView {
                    blocker: b.blocker,
                    attacker: b.attacker,
                })
                .collect(),
        },
    }
}

/// Builds the once-per-game static payload a client needs before it can render
/// anything: who sits where, and the print table its images are keyed by.
#[must_use]
pub fn game_static(
    game_id: String,
    your_seat: PlayerId,
    seats: Vec<baylee_view::SeatIdentity>,
    prints: &[baylee_core::preset::PrintInfo],
) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id,
        your_seat,
        seats,
        prints: prints
            .iter()
            .map(|p| baylee_view::PrintEntry {
                scryfall_id: p.scryfall_id.to_string(),
                lang: p.lang.clone(),
                finish: match p.finish {
                    baylee_core::preset::Finish::Foil => baylee_view::Finish::Foil,
                    baylee_core::preset::Finish::Etched => baylee_view::Finish::Etched,
                    baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
                },
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards::by_oracle_id;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
        SeatSpec,
    };
    use baylee_engine::engine::Engine;
    use baylee_engine::state::CardLookup;

    struct Registry;
    impl CardLookup for Registry {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn island() -> CardIndex {
        by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .unwrap()
            .index
    }

    fn print_info(lang: &str, finish: Finish) -> PrintInfo {
        PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: lang.to_string(),
            finish,
        }
    }

    /// A deck holding the same card in three different printings, plus one
    /// copy of each already on the battlefield.
    fn mixed_print_preset() -> GamePreset {
        let mut deck: Vec<DeckEntry> = Vec::new();
        for i in 0..60u16 {
            deck.push(DeckEntry {
                card: island(),
                print: PrintRef::new(i % 3),
            });
        }
        let seat = |battlefield: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            deck: deck.clone(),
            sideboard: vec![],
            starting_life: None,
            starting_hand: Some(vec![
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(0),
                },
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(2),
                },
            ]),
            starting_battlefield: battlefield,
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 5,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![
                print_info("EN", Finish::Normal),
                print_info("DE", Finish::Foil),
                print_info("JA", Finish::Etched),
            ],
            seats: vec![
                seat(vec![
                    DeckEntry {
                        card: island(),
                        print: PrintRef::new(1),
                    },
                    DeckEntry {
                        card: island(),
                        print: PrintRef::new(2),
                    },
                ]),
                seat(vec![]),
            ],
        }
    }

    /// The whole point of `PrintRef`: two copies of the *same* card in one
    /// deck can be different printings, and the client has to be told which
    /// is which. The engine never interprets the ref — it carries it — so
    /// this test follows one deck entry all the way to the seat view.
    #[test]
    fn the_same_card_in_two_printings_stays_two_printings_in_the_view() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let view = player_view(engine.state(), seat, None, 0);

        let battlefield: Vec<u16> = view
            .battlefield
            .iter()
            .filter(|o| o.controller == seat)
            .filter_map(|o| o.card.map(|c| c.print.get()))
            .collect();
        assert_eq!(
            battlefield,
            vec![1, 2],
            "the battlefield lost the printings the preset asked for"
        );

        let hand: Vec<u16> = view.hand.iter().map(|o| o.card.print.get()).collect();
        assert_eq!(hand, vec![0, 2], "the hand lost its printings");

        // Same rules identity throughout — only the printing differs.
        assert!(
            view.hand.iter().all(|o| o.card.index == island()),
            "print refs must not disturb card identity"
        );
    }

    /// The print table is a per-game payload: the view carries indices, and
    /// `GameStatic` carries what they mean. A client that only got the
    /// indices could not fetch an image.
    #[test]
    fn the_static_payload_carries_what_a_print_ref_points_at() {
        let preset = mixed_print_preset();
        let statics = game_static("g1".into(), PlayerId::new(0), vec![], &preset.prints);
        assert_eq!(statics.prints.len(), 3);
        assert_eq!(statics.prints[1].lang, "DE");
        assert!(matches!(
            statics.prints[1].finish,
            baylee_view::Finish::Foil
        ));
        assert!(matches!(
            statics.prints[2].finish,
            baylee_view::Finish::Etched
        ));
        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    }

    /// A token has no printing, so `card` is `None` and a client has nothing
    /// to fetch an image with. The token id is the handle that replaces it:
    /// it survives the projection into the view, and it resolves back to the
    /// definition the engine created the object from.
    #[test]
    fn a_token_reaches_the_client_with_the_handle_its_art_is_keyed_on() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }

        // Every object the view can project; a card-backed one carries a
        // printing and no token id, and the two are mutually exclusive.
        let view = player_view(engine.state(), PlayerId::new(0), None, 1);
        for object in &view.battlefield {
            assert!(
                object.card.is_none() || object.token.is_none(),
                "{} claims to be both a printing and a token",
                object.name
            );
        }

        // And the id round-trips: whatever the view says, the registry can
        // name it. A token filed under `u16::MAX` — one defined in a card
        // file instead of the registry — would fail here.
        for id in 0..u16::try_from(baylee_cards::tokens::ALL.len()).expect("registry fits") {
            let token = baylee_cards::tokens::by_token_id(id).expect("id names a token");
            assert_eq!(baylee_cards::tokens::token_id(token), id);
        }
    }

    /// A card keeps its printing when it changes zone: the ref lives on the
    /// object, not on the zone it happens to be in. The land is played for
    /// real rather than moved by hand, so the whole cast path is covered.
    #[test]
    fn a_printing_survives_a_zone_change() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let seat = PlayerId::new(0);

        // Walk to seat 0's main phase, where a land may be played.
        for _ in 0..30 {
            if let Pending::Priority { player, legal } = engine.pending()
                && *player == seat
                && !legal.lands.is_empty()
            {
                break;
            }
            let Pending::Priority { player, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending())
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority")
        };
        let card = *legal.lands.first().expect("a land in hand to play");
        let print_before = engine
            .state()
            .object(card)
            .and_then(|o| o.card)
            .expect("a card-backed object")
            .print;
        engine
            .apply(seat, PlayerAction::PlayLand { card })
            .expect("playing a land from hand is legal");

        let view = player_view(engine.state(), seat, None, 1);
        let played = view
            .battlefield
            .iter()
            .find(|o| o.id == card)
            .expect("the land reached the battlefield");
        assert_eq!(
            played.card.expect("card-backed").print,
            print_before,
            "the printing was lost on the way to the battlefield"
        );
        assert!(
            !view.hand.iter().any(|o| o.id == card),
            "the land is still shown in hand"
        );
    }
    /// Every object id that appears anywhere in a view.
    fn ids_in(view: &baylee_view::PlayerView) -> Vec<ObjectId> {
        let mut ids: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
        for zone in [&view.battlefield, &view.stack] {
            ids.extend(zone.iter().map(|o| o.id));
        }
        for per_seat in [&view.graveyards, &view.exile, &view.command] {
            for zone in per_seat {
                ids.extend(zone.iter().map(|o| o.id));
            }
        }
        ids.extend(view.combat.attackers.iter().map(|a| a.creature));
        ids.extend(view.combat.blockers.iter().map(|b| b.blocker));
        ids
    }

    /// The opponent's hand is a number. Not a list the client is trusted to
    /// hide, not ids with the names stripped — a count, with no field the
    /// contents could travel in.
    #[test]
    fn an_opponents_hand_is_a_count_and_nothing_else() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let view = player_view(engine.state(), me, None, 1);

        let their_hand = engine.state().zones.list(ZoneLocation::Hand(them));
        assert!(!their_hand.is_empty(), "the opponent holds cards");
        assert_eq!(
            view.seat(them).map(|s| s.hand_count),
            Some(their_hand.len() as u32),
            "the count is what a client gets"
        );
        let visible = ids_in(&view);
        for id in their_hand {
            assert!(
                !visible.contains(id),
                "an opponent's hand card reached seat 0's view: {id:?}"
            );
        }
    }

    /// Nobody's library is in the view — not even the viewing seat's own.
    /// A player who could read their own library order would know every
    /// draw, which is the same leak wearing a friendlier hat.
    #[test]
    fn no_library_card_reaches_any_view() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            let view = player_view(engine.state(), seat, None, 1);
            let visible = ids_in(&view);
            for owner in [PlayerId::new(0), PlayerId::new(1)] {
                let library = engine.state().zones.list(ZoneLocation::Library(owner));
                assert!(
                    library.len() > 40,
                    "the library should still be nearly whole"
                );
                for id in library {
                    assert!(
                        !visible.contains(id),
                        "a library card reached {seat:?}'s view: {id:?}"
                    );
                }
                assert_eq!(
                    view.seat(owner).map(|s| s.library_count),
                    Some(library.len() as u32)
                );
            }
        }
    }

    /// Two seats looking at the same battlefield see different things when a
    /// permanent is face down (CR 707.2): its controller knows what they
    /// played, everyone else gets a blank with no card identity at all.
    #[test]
    fn a_face_down_permanent_is_blank_to_everyone_but_its_controller() {
        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
        engine
            .state_mut_dev()
            .object_mut(land)
            .expect("the permanent is there")
            .status
            .insert(baylee_engine::object::Status::FACE_DOWN);

        let mine = player_view(engine.state(), me, None, 1);
        let theirs = player_view(engine.state(), them, None, 1);
        let of = |v: &baylee_view::PlayerView| {
            v.battlefield
                .iter()
                .find(|o| o.id == land)
                .expect("the permanent is on the shared battlefield")
                .clone()
        };
        assert!(
            of(&mine).card.is_some(),
            "its controller knows what they played"
        );
        assert!(
            of(&theirs).card.is_none(),
            "the opponent was handed the card identity of a face-down permanent"
        );
        assert_eq!(of(&theirs).name, "Face-down");
    }
}
