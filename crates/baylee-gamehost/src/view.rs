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

use baylee_ai::pending_player;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::Pending;
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::turn::{Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::{Zone, ZoneLocation};
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
        // `TargetRef` has had a `Player` arm since the view was written and
        // never carried one, because the engine's target list was objects
        // only. A burn spell aimed at a face now says so on the stack.
        targets: obj
            .targets
            .iter()
            .map(|t| TargetRef::Object(*t))
            .chain(obj.target_players.iter().map(TargetRef::Player))
            .collect(),
        stack_item: stack_item(obj),
        summoning_sick: obj.kind == ObjectKind::Permanent
            && baylee_engine::combat::summoning_sick(state, obj),
        // Permanents only, for the same reason as `summoning_sick`: nothing
        // else can be tapped for it. The engine's own offer reads the grant
        // through the same function, so what the planner is told a land makes
        // is what the engine will hand out when it is tapped.
        granted_mana: (obj.kind == ObjectKind::Permanent)
            .then(|| granted_mana(state, id))
            .flatten(),
    })
}

/// The mana a granted ability lets `id` make, when it is one a client's
/// planner can use.
///
/// This crate is the one that can see both halves — the engine's effect table
/// and the DSL that says what an `AddMana` produces — which is the same reason
/// `token` is resolved here. It is not hidden information: the grant comes
/// from a permanent on the battlefield and the ability is already offered in
/// `LegalActions` to whoever may activate it.
/// The **first** grant that is a mana ability, which is the one a planner can
/// use: a permanent may be granted several (Urza's Saga is granted two), and
/// the plan taps it once either way.
fn granted_mana(state: &GameState, id: ObjectId) -> Option<baylee_view::GrantedMana> {
    baylee_engine::effects::granted_activated(state, id)
        .take(baylee_engine::choice::GRANTED_SLOTS as usize)
        .enumerate()
        .filter(|(_, g)| g.mana_ability)
        .find_map(|(slot, g)| {
            let mana = baylee_cards_dsl::simple_mana(&g.cost, g.effects)?;
            Some(baylee_view::GrantedMana {
                slot: u32::try_from(slot).unwrap_or(u32::MAX),
                colors: mana.colors,
                amount: mana.amount,
            })
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

/// The objects a pending choice puts in front of the seat it is asking.
///
/// Only the choices that can name a *hidden* object are listed. Combat is
/// not: [`Pending::ChooseAttackers`] and [`Pending::ChooseBlockers`] name
/// creatures, and every creature in a declaration is on the battlefield, which
/// the view already carries in full. Neither is [`Pending::YesNo`]'s miracle
/// card — a miracle is revealed from the hand it was drawn into, and that is
/// the asking seat's own hand.
const fn offered(pending: &Pending) -> &[ObjectId] {
    match pending {
        Pending::ChooseCards { options, .. }
        | Pending::ChooseTargets { options, .. }
        | Pending::LegendChoice { options, .. } => options.as_slice(),
        Pending::OrderObjects { objects, .. } => objects.as_slice(),
        _ => &[],
    }
}

/// Whether `seat`'s view already carries this object somewhere.
///
/// The zones a view sends in full are the public ones plus the seat's own
/// hand; a library, a sideboard and somebody else's hand are counts. An
/// object in one of those is an object the client has no other way to draw,
/// which is exactly what [`PlayerView::looking_at`] is for.
const fn shown_elsewhere(obj: &GameObject, seat: PlayerId) -> bool {
    match obj.zone {
        Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile | Zone::Command => true,
        Zone::Hand => match obj.zone_owner {
            Some(owner) => owner.get() == seat.get(),
            None => false,
        },
        Zone::Library | Zone::OutsideGame => false,
    }
}

/// The hidden cards this seat is being shown, if any.
///
/// The entitlement rule is one sentence and it is the whole safety argument:
/// **an object the engine asks you about is an object you may see.** A search
/// is only offered to the searcher, a scry only to the scrying player, and
/// the engine has already filtered both down to what that player is allowed
/// to look at — so this function adds no judgement of its own beyond checking
/// that the question is addressed to `seat`.
///
/// It is deliberately not a memory. The list is rebuilt from the outstanding
/// choice on every view, so a card stops being visible the instant the choice
/// is answered, and there is no place for one to linger.
fn looking_at(state: &GameState, seat: PlayerId, pending: Option<&Pending>) -> Vec<PublicObject> {
    let Some(pending) = pending else {
        return Vec::new();
    };
    if pending_player(pending) != Some(seat) {
        return Vec::new();
    }
    offered(pending)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|obj| !shown_elsewhere(obj, seat))
        })
        .filter_map(|id| public_object(state, *id, seat))
        .collect()
}

/// Builds the hidden-information-filtered view of `state` for `seat`.
///
/// `priority` is the seat that currently holds priority, which the engine
/// tracks in its pending choice rather than in the state itself.
///
/// `pending` is the outstanding choice, and it is here for one reason:
/// [`PlayerView::looking_at`]. A tutor, a scry and a revealed hand all ask a
/// seat about objects that are in no zone the view carries, so the choice
/// itself is what decides which hidden objects this seat may see. Pass `None`
/// and the view is exactly what it was before — nothing else reads it.
///
/// `held` is whether this seat's own standing order is currently withholding
/// its priority, and it is a parameter for the same reason `priority` is: a
/// hold lives in the engine's `SeatAutomation`, not in the `GameState` this
/// function is handed, so only the caller can read it. Pass
/// `engine.automation(seat).hold.suppresses()`.
#[must_use]
pub fn player_view(
    state: &GameState,
    seat: PlayerId,
    priority: Option<PlayerId>,
    seq: u64,
    pending: Option<&Pending>,
    held: bool,
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
        priority_held: held,
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
        looking_at: looking_at(state, seat, pending),
    }
}

/// Builds the once-per-game static payload a client needs before it can render
/// anything: who sits where, and the print table its images are keyed by.
///
/// `shown` decides, entry by entry, whether this seat has earned the printing.
/// The table is shared by the whole game and deduplicated per card, so a seat
/// handed all of it would be handed the union of every deck at the table. An
/// entry the seat has not earned is `None` rather than absent: the index *is*
/// the [`PrintRef`](baylee_core::ids::PrintRef), and renumbering it would
/// change what every object in every view points at.
#[must_use]
pub fn game_static(
    game_id: String,
    your_seat: PlayerId,
    seats: Vec<baylee_view::SeatIdentity>,
    prints: &[baylee_core::preset::PrintInfo],
    shown: &[bool],
) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id,
        your_seat,
        seats,
        prints: prints
            .iter()
            .enumerate()
            .map(|(i, p)| {
                shown
                    .get(i)
                    .copied()
                    .unwrap_or(false)
                    .then(|| baylee_view::PrintEntry {
                        scryfall_id: p.scryfall_id.to_string(),
                        lang: p.lang.clone(),
                        finish: match p.finish {
                            baylee_core::preset::Finish::Foil => baylee_view::Finish::Foil,
                            baylee_core::preset::Finish::Etched => baylee_view::Finish::Etched,
                            baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
                        },
                    })
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
            capabilities: baylee_core::preset::SeatCapabilities {
                dev_commands: true,
                see_hidden: false,
            },
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
        let view = player_view(engine.state(), seat, None, 0, None, false);

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
        let shown = vec![true; preset.prints.len()];
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &shown,
        );
        assert_eq!(statics.prints.len(), 3);
        let entry = |i: u16| statics.print(PrintRef::new(i)).expect("shown");
        assert_eq!(entry(1).lang, "DE");
        assert!(matches!(entry(1).finish, baylee_view::Finish::Foil));
        assert!(matches!(entry(2).finish, baylee_view::Finish::Etched));
        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    }

    /// A printing this seat has not been shown is a hole in the table, not a
    /// shorter table: the index is the `PrintRef` every object points at.
    #[test]
    fn a_printing_a_seat_has_not_seen_is_a_hole_not_a_gap() {
        let preset = mixed_print_preset();
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &[true, false, true],
        );
        assert_eq!(statics.prints.len(), 3, "the indices do not move");
        assert!(statics.print(PrintRef::new(0)).is_some());
        assert!(statics.print(PrintRef::new(1)).is_none());
        assert!(matches!(
            statics.print(PrintRef::new(2)).map(|p| p.finish),
            Some(baylee_view::Finish::Etched)
        ));
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
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false);
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

        let view = player_view(engine.state(), seat, None, 1, None, false);
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
        let view = player_view(engine.state(), me, None, 1, None, false);

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
            let view = player_view(engine.state(), seat, None, 1, None, false);
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
        // The test preset grants seat 0 dev commands; a lobby game grants
        // nobody any, which is what makes this the harness and not a hole.
        engine
            .dev_state_mut(me)
            .expect("the test preset grants dev commands")
            .object_mut(land)
            .expect("the permanent is there")
            .status
            .insert(baylee_engine::object::Status::FACE_DOWN);

        let mine = player_view(engine.state(), me, None, 1, None, false);
        let theirs = player_view(engine.state(), them, None, 1, None, false);
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

    /// A search offered to `player`, over `options`.
    fn search(player: PlayerId, options: Vec<ObjectId>) -> Pending {
        Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
        }
    }

    /// The first `n` cards of a seat's library, as object ids.
    fn library(engine: &Engine<Registry>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(seat))
            .iter()
            .take(n)
            .copied()
            .collect()
    }

    /// A tutor hands a seat object ids out of its own library. Every other
    /// zone the client can draw from is in the view already; these are in
    /// none of them, so without `looking_at` the dialog is a row of blanks
    /// and the choice cannot be answered at all.
    #[test]
    fn a_searching_seat_is_shown_the_cards_it_was_offered() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let offered = library(&engine, seat, 3);
        let pending = search(seat, offered.clone());

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false);
        let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        assert_eq!(
            shown, offered,
            "the searcher was not shown what it was asked about"
        );
        assert!(
            view.looking_at.iter().all(|o| o.card.is_some()),
            "a card offered out of a library arrived without its identity"
        );
    }

    /// The entitlement is the question, not the game state: the seat being
    /// asked sees the search, and the table does not. This is the sentence
    /// the whole field rests on, so it is the one with a test.
    #[test]
    fn nobody_else_is_shown_another_seat_s_search() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let searcher = PlayerId::new(0);
        let pending = search(searcher, library(&engine, searcher, 3));

        let theirs = player_view(
            engine.state(),
            PlayerId::new(1),
            None,
            0,
            Some(&pending),
            false,
        );
        assert!(
            theirs.looking_at.is_empty(),
            "an opponent was shown the cards a searching seat is looking through"
        );
    }

    /// And it is not a memory. The list is rebuilt from the outstanding
    /// choice every time, so the moment the question is gone the cards are
    /// gone with it — there is nowhere for one to linger.
    #[test]
    fn a_card_stops_being_shown_when_the_question_ends() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);

        let view = player_view(engine.state(), seat, None, 0, None, false);
        assert!(
            view.looking_at.is_empty(),
            "a view with no pending choice was still showing cards"
        );
    }

    /// Most choices name things that are already on the table, and those must
    /// not arrive twice: a client that drew `looking_at` as a dialog would
    /// open one over an ordinary "target creature".
    #[test]
    fn an_offer_of_things_already_in_view_shows_nothing_extra() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let battlefield: Vec<ObjectId> =
            engine.state().zones.list(ZoneLocation::Battlefield).clone();
        assert!(!battlefield.is_empty(), "the preset seats a battlefield");
        let pending = Pending::ChooseTargets {
            player: seat,
            options: battlefield,
            player_options: vec![],
            min: 1,
            max: 1,
        };

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false);
        assert!(
            view.looking_at.is_empty(),
            "objects the view already carries were repeated as things being shown"
        );
    }

    /// A seat earns a printing by seeing the card, and a card out of a
    /// library is a card it now sees. Without this the print table has no
    /// entry for it and the dialog draws rectangles.
    #[test]
    fn a_card_being_shown_earns_its_printing() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let pending = search(seat, library(&engine, seat, 3));

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false);
        for object in &view.looking_at {
            let print = object
                .card
                .expect("a library card is known to its owner")
                .print;
            assert!(
                view.prints().any(|p| p == print),
                "a card being shown did not earn its printing"
            );
        }
    }
    /// A seat's own hand is in its view already, so being asked about it
    /// shows nothing twice. This is the arm of [`shown_elsewhere`] with a
    /// judgement in it: hand is the one zone whose visibility depends on
    /// whose hand it is.
    #[test]
    fn a_seat_asked_about_its_own_hand_is_shown_nothing_extra() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let hand = engine.state().zones.list(ZoneLocation::Hand(seat)).clone();
        assert!(!hand.is_empty(), "the preset deals a starting hand");
        let pending = Pending::ChooseCards {
            player: seat,
            options: hand,
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::PutBackOnTop,
        };

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false);
        assert!(
            view.looking_at.is_empty(),
            "a seat's own hand was repeated as something it is being shown"
        );
    }

    /// And the other side of the same arm, which is where the whole rule is
    /// worth its cost: a discard-at-random or a Thoughtseize asks one seat
    /// about *another* seat's hand. Those cards are hidden from everyone by
    /// default, and the question is what entitles this seat to them — so the
    /// asked seat sees them, in full, and only while it is asked.
    #[test]
    fn a_seat_asked_about_another_hand_is_shown_it() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
        assert!(!hand.is_empty(), "the preset deals a starting hand");
        let pending = Pending::ChooseCards {
            player: me,
            options: hand.clone(),
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        };

        let view = player_view(engine.state(), me, None, 0, Some(&pending), false);
        let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        assert_eq!(
            shown, hand,
            "the asked seat was not shown the hand in question"
        );
        assert!(
            view.looking_at.iter().all(|o| o.card.is_some()),
            "a card this seat is being asked about arrived without its identity"
        );

        // The owner of that hand is being asked nothing, and is shown nothing.
        let theirs = player_view(engine.state(), them, None, 0, Some(&pending), false);
        assert!(
            theirs.looking_at.is_empty(),
            "a seat not being asked was handed a list anyway"
        );
    }
    /// A land under a Chromatic Lantern taps for any colour, and there is no
    /// card anywhere a client could read that off — the ability exists only
    /// in the effect table. It is projected for the same reason as an
    /// animated land's types: without it a client's mana planner counts that
    /// land for nothing and the player taps it by hand.
    ///
    /// The opponent's land in the same test is the half that matters as much:
    /// the grant says "lands *you* control", and a projection that ignored
    /// the filter would offer the planner a land the engine refuses.
    #[test]
    fn a_land_under_a_lantern_says_what_it_now_makes() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let lantern = by_oracle_id("539f5396-d99a-417d-a84c-dff7930b5900")
            .expect("Chromatic Lantern is in the pool")
            .index;
        let mut preset = mixed_print_preset();
        let land = DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        };
        preset.seats[0].starting_battlefield = vec![
            land,
            DeckEntry {
                card: lantern,
                print: PrintRef::new(0),
            },
        ];
        preset.seats[1].starting_battlefield = vec![land];

        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false);

        let land_of = |seat: u8| {
            view.battlefield
                .iter()
                .find(|o| {
                    o.controller == PlayerId::new(seat)
                        && o.types.contains(baylee_core::types::TypeSet::LAND)
                })
                .expect("each seat has its land")
        };
        let granted = land_of(0)
            .granted_mana
            .as_ref()
            .expect("the Lantern grants the land an ability");
        assert_eq!(granted.amount, 1, "one mana, of a colour it will ask for");
        assert_eq!(
            granted.colors.len(),
            5,
            "any colour, and the client has to know which five"
        );

        assert!(
            land_of(1).granted_mana.is_none(),
            "the grant is `lands you control` and the opponent is not you"
        );
        let lantern_itself = view
            .battlefield
            .iter()
            .find(|o| o.types.contains(baylee_core::types::TypeSet::ARTIFACT))
            .expect("the Lantern is on the battlefield");
        assert!(
            lantern_itself.granted_mana.is_none(),
            "the Lantern's own mana ability is printed on it and is not a grant"
        );

        // And the half that makes the projection worth anything: the engine
        // offers this exact land under this exact handle. A view that said a
        // land makes mana the engine will not hand out is worse than one that
        // said nothing — the planner would tap it and the payment would fail.
        let land = land_of(0).id;
        for _ in 0..30 {
            let Pending::Priority { player, legal } = engine.pending().clone() else {
                break;
            };
            if player == PlayerId::new(0) {
                assert!(
                    legal
                        .abilities
                        .contains(&(land, baylee_engine::choice::GRANTED_ABILITY)),
                    "the engine offers the granted ability the view described"
                );
                assert!(
                    legal.mana_abilities.contains(&land),
                    "and offers it as a mana ability, which is why it needs no stack"
                );
                return;
            }
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        panic!("seat 0 never got priority");
    }
}
