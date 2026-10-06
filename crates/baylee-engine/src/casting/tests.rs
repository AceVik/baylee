use super::*;
use crate::effects::{ContinuousEffect, EffectFilter};
use crate::state::{CardLookup, Commander};
use baylee_cards_dsl::{Duration, Filter, Modifier};
use baylee_core::ids::{CardIndex, EffectId, ObjectId, SubtypeId};
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
    SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn me() -> PlayerId {
    PlayerId::new(0)
}
fn them() -> PlayerId {
    PlayerId::new(1)
}

fn state() -> GameState {
    let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("registry contains Forest")
        .index;
    let entry = DeckEntry {
        card: forest,
        print: baylee_core::ids::PrintRef::new(0),
    };
    let seat = || SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: SeatCapabilities::default(),
        deck: (0..60).map(|_| entry).collect(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed: 13,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        seats: vec![seat(), seat()],
    };
    GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
}

fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let name = state.names.intern(name);
    state.create_bare(
        owner,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    )
}

/// A land with the given basic types and nothing else.
fn basic_land(state: &mut GameState, name: &str, types: &[SubtypeId]) -> ObjectId {
    let id = permanent(state, me(), name);
    let base = state.object_mut(id).expect("just made it").base_mut();
    base.types = TypeSet::LAND;
    for t in types {
        base.subtypes.insert(*t);
    }
    id
}

fn register(state: &mut GameState, controller: PlayerId, modifier: Modifier) {
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 1,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(&Filter::This),
        modifier,
    });
}

/// Puts the state in `player`'s first main phase with an empty stack —
/// the one moment a sorcery may be cast, so every refusal below is one
/// thing changed from here.
fn main_phase_of(state: &mut GameState, player: PlayerId) {
    state.turn.phase = Phase::FirstMain;
    state.turn.step = crate::turn::Step::Main;
    state.turn.active = player;
}

fn no_keywords() -> baylee_cards_dsl::KeywordSet {
    baylee_cards_dsl::KeywordSet::EMPTY
}

/// CR 117.1a: an instant whenever its controller has priority, and a
/// noninstant only in its controller's own main phase with the stack
/// empty. Three things make a sorcery illegal and each of them alone is
/// enough, which is why they are asked one at a time — and the second
/// main phase is the fourth row, because the permission is read off the
/// *phase* and `Step::Main` cannot tell the two apart.
#[test]
fn a_sorcery_needs_all_three_of_the_permissions_an_instant_needs_none_of() {
    let mut state = state();
    main_phase_of(&mut state, me());
    let sorcery = |state: &GameState| timing_allows(state, me(), TypeSet::SORCERY, no_keywords());
    let instant = |state: &GameState| timing_allows(state, me(), TypeSet::INSTANT, no_keywords());

    assert!(sorcery(&state), "your own main phase, stack empty");
    assert!(instant(&state));

    state.turn.phase = Phase::SecondMain;
    assert!(sorcery(&state), "and the other main phase is one too");

    state.turn.phase = Phase::Beginning;
    assert!(!sorcery(&state), "not a main phase");
    assert!(instant(&state), "an instant does not care which phase");

    main_phase_of(&mut state, them());
    assert!(!sorcery(&state), "somebody else's main phase");
    assert!(instant(&state));

    main_phase_of(&mut state, me());
    let name = state.names.intern("Something");
    state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
    assert!(!sorcery(&state), "the stack is not empty");
    assert!(instant(&state), "which is exactly when an instant is for");
}

/// Flash (CR 702.8a) puts a card in the instant group whatever its types
/// say, and Teferi's `+1` does the same for that player's **sorceries**
/// only. Two halves a "your spells have flash" reading would get wrong:
/// it is not granted to a creature, and it is not granted to the player
/// across the table.
#[test]
fn flash_and_a_granted_flash_are_read_from_two_different_places() {
    let mut state = state();
    main_phase_of(&mut state, them());
    let flash = baylee_cards_dsl::KeywordSet::FLASH;

    assert!(
        !timing_allows(&state, me(), TypeSet::CREATURE, no_keywords()),
        "a creature on somebody else's turn"
    );
    assert!(
        timing_allows(&state, me(), TypeSet::CREATURE, flash),
        "and the same creature with flash"
    );

    assert!(!timing_allows(
        &state,
        me(),
        TypeSet::SORCERY,
        no_keywords()
    ));
    register(&mut state, them(), Modifier::SorceriesHaveFlash);
    assert!(
        !timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()),
        "the +1 an opponent activated is not mine"
    );

    register(&mut state, me(), Modifier::SorceriesHaveFlash);
    assert!(
        timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()),
        "and the one I activated is"
    );
    assert!(
        !timing_allows(&state, me(), TypeSet::CREATURE, no_keywords()),
        "it says sorceries, and a creature is not one"
    );
}

/// Teferi's static is asked **first** and beats flash: an opponent's
/// instant is pulled back to sorcery speed whatever it says. What it
/// leaves them is exactly what sorcery speed is — their own main phase
/// with an empty stack — and its own controller is not their own
/// opponent, which is the row that says the question is asked per seat.
#[test]
fn a_sorcery_speed_lock_beats_flash_and_spares_its_controller() {
    let mut state = state();
    main_phase_of(&mut state, them());
    let flash = baylee_cards_dsl::KeywordSet::FLASH;

    assert!(
        timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
        "before the lock, an instant on anybody's turn"
    );
    register(&mut state, them(), Modifier::OpponentsCastAsSorcery);
    assert!(
        !timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
        "and under it, not on theirs"
    );
    assert!(
        !timing_allows(&state, me(), TypeSet::CREATURE, flash),
        "flash does not get out from under it"
    );

    main_phase_of(&mut state, me());
    assert!(
        timing_allows(&state, them(), TypeSet::INSTANT, no_keywords()),
        "the seat that controls the lock is not locked by it — and it is \
         not their turn, so nothing else explains this"
    );
    assert!(
        timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
        "the lock leaves an opponent their own main phase"
    );
    assert!(timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()));
}

/// A card of the given types in `owner`'s hand, carrying nothing else.
fn card_in_hand(state: &mut GameState, owner: PlayerId, name: &str, types: TypeSet) -> ObjectId {
    let name = state.names.intern(name);
    let id = state.create_bare(owner, ObjectKind::Card, name, ZoneLocation::Hand(owner));
    state.object_mut(id).expect("just made it").base_mut().types = types;
    id
}

/// A cast an effect forbids outright is a different answer from a
/// sorcery-speed lock, which is the whole reason `OpponentsCantCast` is
/// its own variant: every row below is taken in `me()`'s own main phase
/// with an empty stack, the one moment `timing_allows` says yes to
/// everything.
///
/// Three rows, each alone the point. The spell is castable before the
/// effect exists — so the refusal is the effect and not the setup. It is
/// refused while an opponent's effect names it. And it is **not**
/// refused for the seat that controls the effect, because "your
/// opponents" is asked per seat and never reaches you.
#[test]
fn a_cast_lock_is_asked_per_seat_and_not_of_its_own_controller() {
    let mut state = state();
    main_phase_of(&mut state, me());
    let spell = card_in_hand(&mut state, me(), "Probe", TypeSet::INSTANT);

    let obj = state.object(spell).expect("just made it");
    assert!(
        !cast_is_forbidden(&state, me(), obj),
        "nothing forbids it yet"
    );

    register(
        &mut state,
        them(),
        Modifier::OpponentsCantCast(&Filter::Any),
    );
    let obj = state.object(spell).expect("still there");
    assert!(
        cast_is_forbidden(&state, me(), obj),
        "an opponent's lock reaches this seat"
    );
    assert!(
        !cast_is_forbidden(&state, them(), obj),
        "and not the seat that controls it — \"your opponents\" is asked \
         per seat"
    );
}

/// The filter the lock carries is read, and is read from the **effect's**
/// controller: Ranger-Captain of Eos forbids noncreature spells, so a
/// creature card in the same hand is still castable. Without this the
/// variant would be a bit rather than a sentence, and Silence and the
/// Ranger-Captain would be the same card.
#[test]
fn a_cast_lock_forbids_only_what_its_filter_names() {
    let mut state = state();
    main_phase_of(&mut state, me());
    let instant = card_in_hand(&mut state, me(), "Probe", TypeSet::INSTANT);
    let creature = card_in_hand(&mut state, me(), "Bear", TypeSet::CREATURE);
    register(
        &mut state,
        them(),
        Modifier::OpponentsCantCast(&Filter::NONCREATURE),
    );

    assert!(
        cast_is_forbidden(&state, me(), state.object(instant).expect("there")),
        "an instant is a noncreature spell"
    );
    assert!(
        !cast_is_forbidden(&state, me(), state.object(creature).expect("there")),
        "and a creature is not"
    );
}

#[test]
fn disguise_uses_creature_characteristics_for_restrictions_and_payment() {
    let mut state = state();
    main_phase_of(&mut state, me());
    let card = card_in_hand(&mut state, me(), "land", TypeSet::LAND);
    register(
        &mut state,
        them(),
        Modifier::OpponentsCantCast(&Filter::NONCREATURE),
    );
    let original = state.object(card).unwrap();
    let disguise = SpellForm::Disguise.project(original);
    assert!(!crate::eval::matches(
        &Filter::Named("land"),
        &state,
        &disguise,
        me(),
        card
    ));
    assert_eq!(state.names.get(disguise.characteristics().name), "");
    assert!(cast_is_forbidden(&state, me(), original));
    assert!(!cast_is_forbidden(
        &state,
        me(),
        &SpellForm::Disguise.project(original)
    ));
    let id = baylee_core::mana::RestrictionId(42);
    state
        .restriction_info
        .insert(42, (card, &Filter::CREATURE, SpendRider::None));
    state.players[0].mana_pool.add_restricted(RestrictedMana {
        color: ManaColor::Colorless,
        amount: 3,
        flags: ManaFlags::default(),
        restriction: id,
    });
    let cost = ManaCost::parse("{3}");
    assert!(pay_mana_for(&mut state, me(), SpendFor::Spell(card), &cost).is_none());
    assert!(
        pay_mana_for(
            &mut state,
            me(),
            SpendFor::SpellAs(card, SpellForm::Disguise),
            &cost
        )
        .is_some()
    );
    assert!(state.players[0].mana_pool.restricted().is_empty());
    assert_eq!(
        state.object(card).unwrap().characteristics().types,
        TypeSet::LAND,
        "a probe never changes the card"
    );
}

#[test]
fn directed_spending_preserves_restrictions_snow_and_actual_rider_units() {
    for snow in [false, true] {
        for ridden in [false, true] {
            let mut state = state();
            let creature = card_in_hand(&mut state, me(), "Creature", TypeSet::CREATURE);
            let instant = card_in_hand(&mut state, me(), "Instant", TypeSet::INSTANT);
            register(
                &mut state,
                me(),
                Modifier::SpendManaAs {
                    from: ManaColor::White,
                    to: ManaColor::Red,
                },
            );
            state
                .restriction_info
                .insert(42, (creature, &Filter::CREATURE, SpendRider::Uncounterable));
            let mana = RestrictedMana {
                color: ManaColor::White,
                amount: 2,
                flags: if snow {
                    ManaFlags::SNOW
                } else {
                    ManaFlags::NONE
                },
                restriction: baylee_core::mana::RestrictionId(42),
            };
            if ridden {
                state.players[0].mana_pool.add_ridden(mana);
            } else {
                state.players[0].mana_pool.add_restricted(mana);
                let before = state.players[0].mana_pool.clone();
                for what in [
                    SpendFor::Spell(instant),
                    SpendFor::Ability(creature),
                    SpendFor::Other,
                ] {
                    assert!(
                        pay_mana_for(&mut state, me(), what, &ManaCost::parse("{R}")).is_none()
                    );
                    assert_eq!(state.players[0].mana_pool, before);
                }
            }
            state.players[0].mana_pool.add(ManaColor::Red, 1);
            let cost = ManaCost::parse("{R}");
            let pool = spendable_pool(&state, me(), SpendFor::Spell(creature));
            assert!(affordable(
                &state,
                me(),
                pool.as_ref().unwrap_or(&state.players[0].mana_pool),
                &cost
            ));
            let paid = pay_mana_for(&mut state, me(), SpendFor::Spell(creature), &cost).unwrap();
            assert_eq!(
                paid.as_slice(),
                &[(
                    RestrictedMana { amount: 1, ..mana },
                    creature,
                    SpendRider::Uncounterable
                )]
            );
            let pool = &state.players[0].mana_pool;
            assert_eq!(
                pool.available(ManaColor::Red),
                1,
                "the preferred white unit paid red"
            );
            let remaining = if ridden {
                pool.ridden()
            } else {
                pool.restricted()
            };
            assert_eq!(remaining, &[RestrictedMana { amount: 1, ..mana }]);
            if snow {
                assert!(
                    pay_mana_for(
                        &mut state,
                        me(),
                        SpendFor::Spell(creature),
                        &ManaCost::parse("{R}{S}")
                    )
                    .is_some()
                );
                assert!(state.players[0].mana_pool.is_empty());
            }
        }
    }
}

/// A face with no text, carrying only the three fields these probes read.
fn probe_face(
    convoke: bool,
    delve: bool,
    reduction: Option<baylee_cards_dsl::CostReduction>,
) -> baylee_cards_dsl::FaceDef {
    let mut face = crate::engine::synthetic::land_face("Probe");
    face.convoke = convoke;
    face.delve = delve;
    face.cost_reduction = reduction;
    face
}

fn artifact(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let id = permanent(state, owner, name);
    state.object_mut(id).expect("just made it").base_mut().types = TypeSet::ARTIFACT;
    id
}

fn creature(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let id = permanent(state, owner, name);
    state.object_mut(id).expect("just made it").base_mut().types = TypeSet::CREATURE;
    id
}

fn tap(state: &mut GameState, id: ObjectId) {
    state.set_tapped(id, true);
}

/// Convoke pays with untapped creatures its caster controls (CR
/// 702.51a), and each of those words is a row: a tapped one is not a
/// source, an opponent's is not a source, a land is not one however
/// untapped it is, and a phased-out one does not exist (CR 702.26b).
/// Waterbend pays with the same and with artifacts too (CR 701.67a).
///
/// #229: the two were one walk over creatures and artifacts, and this
/// test asserted it. The walk read the whole battlefield, phased-out
/// permanents included.
///
/// The count is what the offer and the payment both read. They disagreed
/// once and the result was a convoke spell offered exactly when its
/// printed cost was already payable — which is the one case convoke is
/// not for. A waterbend adds nothing to the offer (CR 701.67b).
#[test]
fn convoke_taps_creatures_and_waterbend_artifacts_too_on_one_side() {
    let mut state = state();
    let bear = creature(&mut state, me(), "Bear");
    let mox = artifact(&mut state, me(), "Mox");
    let tapped = creature(&mut state, me(), "Tapped");
    tap(&mut state, tapped);
    let theirs = creature(&mut state, them(), "Theirs");
    tap(&mut state, theirs);
    let untapped_theirs = creature(&mut state, them(), "Also theirs");
    let land = permanent(&mut state, me(), "Land");
    state.object_mut(land).expect("made it").base_mut().types = TypeSet::LAND;
    for phased in [
        creature(&mut state, me(), "Phased creature"),
        artifact(&mut state, me(), "Phased artifact"),
    ] {
        state
            .object_mut(phased)
            .expect("made it")
            .status
            .insert(crate::object::Status::PHASED_OUT);
    }

    assert_eq!(
        convoke_sources(&state, me()),
        vec![bear],
        "an artifact, a tapped one, an opponent's, a land and a phased-out \
         one are none of them"
    );
    assert_eq!(
        waterbend_sources(&state, me()),
        vec![bear, mox],
        "waterbend taps the artifact as well, and nothing else more"
    );
    assert_eq!(
        convoke_sources(&state, them()),
        vec![untapped_theirs],
        "and the other side counts its own"
    );

    // What the keyword is then worth, which is the number the two probes
    // must agree on — and nothing at all on a face that does not print it.
    assert_eq!(
        keyword_reduction(&state, &probe_face(true, false, None), me(), bear),
        1
    );
    assert_eq!(
        keyword_reduction(&state, &probe_face(false, false, None), me(), bear),
        0
    );
    let mut waterbend = probe_face(false, false, None);
    waterbend.waterbend = true;
    assert_eq!(
        keyword_reduction(&state, &waterbend, me(), bear),
        0,
        "a waterbend's taps pay the waterbend and never the printed cost"
    );
}

/// Delve pays with the caster's graveyard (CR 702.66a), one card each,
/// and it stacks with convoke on a face that printed both. Dig Through
/// Time is the pool's only delve card and was offered at eight mana or
/// not at all, with a full graveyard doing nothing.
#[test]
fn delve_counts_a_graveyard_and_adds_to_whatever_else_the_face_prints() {
    let mut state = state();
    let buried: Vec<ObjectId> = (0..3)
        .map(|i| {
            let name = state.names.intern(&format!("Buried {i}"));
            state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Graveyard(me()))
        })
        .collect();
    let name = state.names.intern("Theirs");
    state.create_bare(
        them(),
        ObjectKind::Card,
        name,
        ZoneLocation::Graveyard(them()),
    );
    let bear = creature(&mut state, me(), "Bear");

    assert_eq!(
        keyword_reduction(&state, &probe_face(false, true, None), me(), bear),
        3,
        "my graveyard, and not the table's"
    );
    assert_eq!(
        keyword_reduction(&state, &probe_face(false, true, None), them(), bear),
        1
    );
    assert_eq!(
        keyword_reduction(&state, &probe_face(true, true, None), me(), bear),
        4,
        "a face printing both adds them"
    );
    // Cast out of that graveyard, the spell is on the stack while it is
    // paid for (CR 601.2a) and is not one of the cards it may exile.
    assert_eq!(
        keyword_reduction(&state, &probe_face(false, true, None), me(), buried[0]),
        2,
        "the card being cast is not its own delve"
    );
}

/// A printed reduction is read off the card and asked of the seat:
/// Surgical Metamorph costs `{1}` less if you were not the starting
/// player, so the seat it is *for* is the one the offer used to leave it
/// out for.
#[test]
fn a_printed_reduction_reaches_the_seat_it_was_printed_for() {
    let mut state = state();
    let card = permanent(&mut state, me(), "Probe");
    let face = probe_face(
        false,
        false,
        Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(1)),
    );
    let starter = state.starting_player;
    let other = if starter == me() { them() } else { me() };

    assert_eq!(printed_reduction(&state, &face, starter, card), 0);
    assert_eq!(printed_reduction(&state, &face, other, card), 1);
    assert_eq!(
        printed_reduction(&state, &probe_face(false, false, None), other, card),
        0,
        "and a card that prints no reduction gets none"
    );
}

/// "Costs {1} less … for each creature you control" counts for the seat
/// paying: two creatures of mine take two off my price, and my
/// opponent's one creature takes one off theirs. `each` multiplies.
#[test]
fn a_counted_reduction_takes_generic_mana_per_thing_counted() {
    let mut state = state();
    let card = permanent(&mut state, me(), "Probe");
    creature(&mut state, me(), "Mine");
    creature(&mut state, me(), "Also mine");
    creature(&mut state, them(), "Theirs");
    let per = |each| {
        probe_face(
            false,
            false,
            Some(baylee_cards_dsl::CostReduction::PerCount {
                amount: baylee_cards_dsl::Amount::CountOf {
                    filter: &Filter::YOUR_CREATURE,
                    zone: baylee_cards_dsl::ZoneSel::Battlefield,
                },
                each,
            }),
        )
    };
    assert_eq!(printed_reduction(&state, &per(1), me(), card), 2);
    assert_eq!(printed_reduction(&state, &per(1), them(), card), 1);
    assert_eq!(printed_reduction(&state, &per(2), me(), card), 4);
    let cost = ManaCost::parse("{1}{G}");
    assert_eq!(
        cost.with_less_generic(printed_reduction(&state, &per(1), me(), card)),
        ManaCost::parse("{G}"),
        "generic only, and never below nothing (CR 118.7a)"
    );
}

/// Mycosynth Lattice's third line, which the affordability checks did not
/// read at all — so the spell it made payable was never offered as
/// castable in the first place.
#[test]
fn mana_is_wild_only_while_something_says_so() {
    let mut state = state();
    assert!(!mana_is_wild(&state));
    register(&mut state, me(), Modifier::ManaIsAnyColor);
    assert!(
        mana_is_wild(&state),
        "it is a question about the table and not about a seat"
    );
}

/// CR 305.6 gives a land one mana ability **per** basic type, so a land
/// with two of them is a question rather than an answer — and the two
/// readers say so differently on purpose. `intrinsic_mana` is the
/// single-answer shortcut and stays `None`, because answering on the
/// player's behalf is worse than not answering: Godless Shrine used to
/// tap for white and never for black, whatever the player needed.
/// `intrinsic_mana_colors` is what a caller that *can* ask reads, and it
/// names both.
#[test]
fn a_land_with_two_basic_types_is_a_question_and_not_an_answer() {
    let mut state = state();
    let plains = basic_land(&mut state, "Plains", &[land::PLAINS]);
    let shrine = basic_land(&mut state, "Godless Shrine", &[land::PLAINS, land::SWAMP]);
    let waste = basic_land(&mut state, "Wastes", &[]);
    let bear = permanent(&mut state, me(), "Bear");

    assert_eq!(intrinsic_mana(&state, plains), Some(ManaColor::White));
    assert_eq!(
        intrinsic_mana(&state, shrine),
        None,
        "the shortcut does not pick for the player"
    );
    assert_eq!(intrinsic_mana(&state, waste), None, "no basic type");
    assert_eq!(intrinsic_mana(&state, bear), None, "not a land at all");

    assert_eq!(
        intrinsic_mana_colors(&state, plains),
        vec![ManaColor::White]
    );
    assert_eq!(
        intrinsic_mana_colors(&state, shrine),
        vec![ManaColor::White, ManaColor::Black],
        "both abilities the dual has, in the rules' own order"
    );
    assert!(
        intrinsic_mana_colors(&state, waste).is_empty(),
        "no basic type is no ability at all — Wastes prints its own"
    );
    assert!(intrinsic_mana_colors(&state, bear).is_empty());
}

/// CR 903.8: `{2}` for each previous cast of *this* commander from the
/// command zone, and nothing at all while the card is somewhere else —
/// a commander cast from a hand it was bounced to pays no tax.
#[test]
fn the_commander_tax_is_per_commander_and_only_in_the_command_zone() {
    let mut state = state();
    let name = state.names.intern("General");
    let general = state.create_bare(
        me(),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Command(me()),
    );
    let name = state.names.intern("Partner");
    let partner = state.create_bare(
        me(),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Command(me()),
    );
    state.commanders[me().get() as usize].push(Commander {
        object: general,
        casts: 2,
        answered: 0,
    });
    state.commanders[me().get() as usize].push(Commander {
        object: partner,
        casts: 0,
        answered: 0,
    });

    assert_eq!(commander_tax(&state, me(), general), 4);
    assert_eq!(
        commander_tax(&state, me(), partner),
        0,
        "a partner deck taxes its two independently"
    );
    assert_eq!(
        commander_tax(&state, them(), general),
        0,
        "and it is the caster's own list that is read"
    );

    state
        .move_object(
            general,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("it is bounced to hand");
    assert_eq!(
        commander_tax(&state, me(), general),
        0,
        "the tax is on casting it from the command zone"
    );
}

/// "If you control a commander" is card text rather than a rule, and it
/// reads the marker list: a commander on the battlefield has left the
/// command zone by definition, so asking the zone answers no for exactly
/// the board the card is printed about.
#[test]
fn controlling_a_commander_is_asked_of_the_marker_list() {
    let mut state = state();
    let general = permanent(&mut state, me(), "General");

    assert!(!controls_a_commander(&state, me()), "no commander yet");

    state.commanders[me().get() as usize].push(Commander {
        object: general,
        casts: 0,
        answered: 0,
    });
    assert!(controls_a_commander(&state, me()));
    assert!(
        !controls_a_commander(&state, them()),
        "theirs, not the table's"
    );

    state
        .move_object(
            general,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("it dies");
    assert!(
        !controls_a_commander(&state, me()),
        "a commander in the graveyard is not one you control"
    );
}

/// One land a turn (CR 305.2a), and the extra drops are a fold rather
/// than a flag. The predicate beside it is the one both ends ask — the
/// offer that puts a land in `legal.lands` and the play that refuses an
/// answer nobody offered — so that a limit which stops being one cannot
/// be read two ways.
#[test]
fn a_land_drop_is_counted_in_one_place_for_both_ends() {
    let mut state = state();
    assert_eq!(land_drops_allowed(&state, me()), 1);
    assert!(has_a_land_drop_left(&state, me()));

    state.players[me().get() as usize].lands_played_this_turn = 1;
    assert!(!has_a_land_drop_left(&state, me()));

    register(&mut state, me(), Modifier::ExtraLandDrops(1));
    assert_eq!(land_drops_allowed(&state, me()), 2);
    assert!(has_a_land_drop_left(&state, me()), "Exploration");
    assert_eq!(
        land_drops_allowed(&state, them()),
        1,
        "an extra drop is its controller's"
    );

    register(&mut state, me(), Modifier::ExtraLandDrops(2));
    assert_eq!(land_drops_allowed(&state, me()), 4, "they add up");
}

/// CR 305.1: the hand needs no permission, and the graveyard is one —
/// Crucible of Worlds. `GrantsFlashback` is the neighbouring sentence
/// about a graveyard and says nothing here, because playing a land is
/// not casting a spell.
#[test]
fn a_land_is_played_from_the_hand_and_from_a_graveyard_only_by_permission() {
    let mut state = state();
    assert!(land_zone_open(&state, me(), Zone::Hand));
    assert!(!land_zone_open(&state, me(), Zone::Graveyard));
    assert!(!land_zone_open(&state, me(), Zone::Exile));

    register(&mut state, me(), Modifier::GrantsFlashback);
    assert!(
        !land_zone_open(&state, me(), Zone::Graveyard),
        "flashback is about casting a spell"
    );

    register(&mut state, me(), Modifier::PlayLandsFromGraveyard);
    assert!(land_zone_open(&state, me(), Zone::Graveyard));
    assert!(
        !land_zone_open(&state, them(), Zone::Graveyard),
        "the permission is its controller's"
    );
}
