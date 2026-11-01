//! Actor authentication and exact resource obligations are shared rules primitives.
use super::synthetic::{self, SyntheticLookup};
use super::testkit::{self, RegistryLookup};
use super::*;
use baylee_cards_dsl::prelude::*;
use baylee_core::mana::{ManaColor, ManaFlags, ManaPool, RestrictedMana, RestrictionId};
const ACTOR: PlayerId = PlayerId::new(0);
const SUBJECT: PlayerId = PlayerId::new(1);
const PROBE: u32 = 995_100;
const SPELL: u32 = 995_101;

fn fixture() -> Engine<SyntheticLookup> {
    static ABILITIES: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::ControlPlayerPlayCard {
            player: PlayerRel::Chosen
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    static SPELL_ABILITIES: &[AbilityDef] = &[spell!(
        &[Effect::GainLife {
            amount: Amount::Fixed(2)
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )];
    let probe = synthetic::land(PROBE, "Decision-control probe", ABILITIES);
    let spell = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            mana_cost: mana!("{1}"),
            types: TypeSet::INSTANT,
            ..synthetic::land_face("Resource probe")
        }])),
        abilities: SPELL_ABILITIES,
        ..*synthetic::land(SPELL, "Resource probe", &[])
    }));
    let mut preset = synthetic::preset(115_723, &[PROBE]);
    preset.seats.push(preset.seats[1].clone());
    preset.seats[1].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_core::ids::CardIndex::new(SPELL),
        print: baylee_core::ids::PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![probe, spell])).unwrap();
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    for _ in 0..50 {
        if matches!(engine.pending(), Pending::Priority { player: ACTOR, .. }) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(&mut engine, &pending),
            "unexpected pending: {pending:?}"
        );
    }
    engine
}

#[test]
fn controlled_payment_consumes_generated_snow_and_preserves_existing_plain_units() {
    let mut engine = fixture();
    let card = engine.state.zones.list(ZoneLocation::Hand(SUBJECT))[0];
    let reference = engine.state.source_identity(card).unwrap();
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .state
        .constrained_payments
        .push(crate::constrained_payment::ConstrainedPayment {
            player: SUBJECT,
            card: reference,
            required: ManaPool::new(),
        });
    let before = engine.state.players[1].mana_pool.clone();
    engine.state.players[1]
        .mana_pool
        .add_snow(ManaColor::Blue, 1);
    engine.state.note_constrained_production(SUBJECT, &before);
    assert!(
        casting::pay_mana_for(
            &mut engine.state,
            SUBJECT,
            casting::SpendFor::Spell(card),
            &mana!("{1}")
        )
        .is_some()
    );
    assert_eq!(
        engine.state.players[1].mana_pool.available(ManaColor::Blue),
        1
    );
    assert_eq!(
        engine.state.players[1]
            .mana_pool
            .snow_available(ManaColor::Blue),
        0
    );
    assert!(engine.state.constrained_payments[0].required.is_empty());
}

#[test]
fn restricted_generated_mana_cannot_be_replaced_by_an_existing_unrestricted_unit() {
    let mut engine = fixture();
    let card = engine.state.zones.list(ZoneLocation::Hand(SUBJECT))[0];
    let reference = engine.state.source_identity(card).unwrap();
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .state
        .constrained_payments
        .push(crate::constrained_payment::ConstrainedPayment {
            player: SUBJECT,
            card: reference,
            required: ManaPool::new(),
        });
    let before = engine.state.players[1].mana_pool.clone();
    engine.state.players[1]
        .mana_pool
        .add_restricted(RestrictedMana {
            color: ManaColor::Blue,
            amount: 1,
            flags: ManaFlags::NONE,
            restriction: RestrictionId(991),
        });
    engine.state.note_constrained_production(SUBJECT, &before);
    let before = engine.state.snapshot_hash();
    assert!(
        casting::pay_mana_for(
            &mut engine.state,
            SUBJECT,
            casting::SpendFor::Spell(card),
            &mana!("{1}")
        )
        .is_none()
    );
    assert_eq!(engine.state.snapshot_hash(), before);
}

#[test]
fn control_authentication_refuses_subject_and_third_seat_without_mutation() {
    let mut engine = fixture();
    let source = synthetic::permanents(&engine, PROBE)[0];
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .apply(
            ACTOR,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            ACTOR,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![SUBJECT],
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(&mut engine, &pending),
            "unexpected pending: {pending:?}"
        );
    }
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let before = engine.fingerprint();
    for player in [SUBJECT, PlayerId::new(2)] {
        assert!(
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![options[0]]
                    }
                )
                .is_err()
        );
        assert_eq!(engine.fingerprint(), before);
    }
    engine
        .apply(
            ACTOR,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    assert_eq!(engine.decision_actor(), Some(ACTOR));
    assert_eq!(engine.pending().asked(), Some(SUBJECT));
    assert!(engine.pending_for(ACTOR).is_some());
    assert!(engine.pending_for(SUBJECT).is_none());
    assert_eq!(engine.awaited().iter().collect::<Vec<_>>(), vec![ACTOR]);
    assert!(engine.information_pending_for(SUBJECT).is_some());
    let before = engine.fingerprint();
    let answer = match engine.pending() {
        Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(SUBJECT),
        Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![SUBJECT],
        },
        pending => panic!("unexpected spell target: {pending:?}"),
    };
    for player in [SUBJECT, PlayerId::new(2)] {
        assert!(engine.apply(player, answer.clone()).is_err());
        assert_eq!(engine.fingerprint(), before);
    }
    engine.apply(ACTOR, answer).unwrap();
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    assert_eq!(engine.state.players[1].mana_pool.total(), 0);
}

fn at_controlled_target() -> Engine<SyntheticLookup> {
    let mut engine = fixture();
    let source = synthetic::permanents(&engine, PROBE)[0];
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .apply(
            ACTOR,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            ACTOR,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![SUBJECT],
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending), "{pending:?}");
    }
    let card = engine.state.zones.list(ZoneLocation::Hand(SUBJECT))[0];
    engine
        .apply(
            ACTOR,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
    assert_eq!(engine.pending().asked(), Some(SUBJECT));
    engine
}

#[test]
fn unrelated_payment_preserves_the_generated_obligation_and_its_snow_units() {
    let mut engine = fixture();
    let card = engine.state.zones.list(ZoneLocation::Hand(SUBJECT))[0];
    let reference = engine.state.source_identity(card).unwrap();
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .state
        .constrained_payments
        .push(crate::constrained_payment::ConstrainedPayment {
            player: SUBJECT,
            card: reference,
            required: ManaPool::new(),
        });
    let before = engine.state.players[1].mana_pool.clone();
    engine.state.players[1]
        .mana_pool
        .add_snow(ManaColor::Blue, 1);
    engine.state.note_constrained_production(SUBJECT, &before);
    assert!(casting::pay_mana(&mut engine.state, SUBJECT, &mana!("{1}")));
    assert_eq!(
        engine.state.players[1]
            .mana_pool
            .snow_available(ManaColor::Blue),
        1
    );
    assert_eq!(engine.state.constrained_payments[0].required.total(), 1);
    let before = engine.state.snapshot_hash();
    assert!(!casting::pay_mana(
        &mut engine.state,
        SUBJECT,
        &mana!("{1}")
    ));
    assert_eq!(engine.state.snapshot_hash(), before);
}

#[test]
fn a_controller_leaving_restores_the_subjects_decision_without_changing_its_costs() {
    let mut engine = at_controlled_target();
    engine.apply(ACTOR, PlayerAction::Concede).unwrap();
    assert!(engine.state.has_left(ACTOR));
    assert!(!engine.state.has_left(SUBJECT));
    assert_eq!(engine.decision_actor(), Some(SUBJECT));
    assert!(engine.controlled_players(ACTOR).is_empty());
    assert!(
        engine
            .apply(ACTOR, PlayerAction::ChoosePlayer(SUBJECT))
            .is_err()
    );
    engine
        .apply(SUBJECT, PlayerAction::ChoosePlayer(SUBJECT))
        .unwrap();
    assert_eq!(engine.state.players[1].mana_pool.total(), 0);
    assert!(engine.effect_plays.is_empty());
    assert!(engine.state.constrained_payments.is_empty());
}

#[test]
fn a_subject_leaving_drops_the_cast_and_finishes_the_waiting_instruction() {
    let mut engine = at_controlled_target();
    engine.apply(SUBJECT, PlayerAction::Concede).unwrap();
    assert!(engine.state.has_left(SUBJECT));
    assert!(!engine.state.has_left(ACTOR));
    assert!(engine.effect_plays.is_empty());
    assert!(engine.state.constrained_payments.is_empty());
    assert!(engine.state.zones.stack_is_empty());
    assert_ne!(engine.pending().asked(), Some(SUBJECT));
}

#[test]
fn scheduled_player_control_applies_only_during_the_subjects_turn() {
    let mut engine = fixture();
    engine
        .state
        .turn_controls
        .push(crate::state::PlayerTurnControl {
            controller: ACTOR,
            player: SUBJECT,
            turn: engine.state.turn.number + 1,
            began: None,
        });
    assert_eq!(engine.controlled_actor(SUBJECT), SUBJECT);

    engine.state.turn.number += 1;
    engine.state.turn.active = SUBJECT;
    engine.begin_turn_controls();
    assert_eq!(engine.controlled_actor(SUBJECT), ACTOR);

    engine.state.turn.number += 1;
    engine.state.turn.active = ACTOR;
    engine.begin_turn_controls();
    assert_eq!(engine.controlled_actor(SUBJECT), SUBJECT);
    assert!(engine.state.turn_controls.is_empty());
}

/// Mindslaver's control as `Effect::ControlPlayerNextTurn` schedules it,
/// for `subject`, created during `seat(0)`'s first main phase.
fn scheduled(seats: usize, seed: u64, subject: PlayerId) -> Engine<RegistryLookup> {
    let mut engine = testkit::Duel::table(seed, testkit::basic_forest(), seats).start();
    testkit::keep_mulligans(&mut engine);
    testkit::reach_main_phase(&mut engine, PlayerId::new(0));
    engine
        .state
        .turn_controls
        .push(crate::state::PlayerTurnControl {
            controller: PlayerId::new(0),
            player: subject,
            turn: engine.state.turn.number + 1,
            began: None,
        });
    engine
}

/// Plays on until the next turn has begun and `during` has run in it, and
/// says whose turn it is and who answers for `subject`.
fn next_turn(
    engine: &mut Engine<RegistryLookup>,
    subject: PlayerId,
    during: impl FnOnce(&mut Engine<RegistryLookup>),
) -> (PlayerId, PlayerId) {
    let now = engine.state.turn.number;
    testkit::pass_until(engine, |e| e.state().turn.number > now);
    during(engine);
    (engine.state.turn.active, engine.controlled_actor(subject))
}

/// "This effect applies to the next turn that the affected player actually
/// takes" (CR 722.1): at a table of four, seat 0's Mindslaver on seat 3
/// waits through seats 1 and 2, controls seat 3's turn, and has ended as
/// seat 0's begins. The global turn number passing the one the control was
/// created for is no reason to drop it.
#[test]
fn a_scheduled_control_waits_through_the_other_players_turns() {
    let subject = PlayerId::new(3);
    let mut engine = scheduled(4, 7_221, subject);
    let turns: Vec<_> = (0..4)
        .map(|_| next_turn(&mut engine, subject, |_| {}))
        .collect();
    let seat = PlayerId::new;
    assert_eq!(
        turns,
        [
            (seat(1), subject),
            (seat(2), subject),
            (subject, seat(0)),
            (seat(0), subject),
        ]
    );
    assert!(
        engine.state.turn_controls.is_empty(),
        "the control ended as the next turn began"
    );
}

/// Extra turns are turns too (CR 500.7): two the controller takes before
/// the subject's are not the subject's, and the control waits through them.
#[test]
fn a_scheduled_control_waits_through_the_controllers_extra_turns() {
    let mut engine = scheduled(2, 7_222, SUBJECT);
    engine.state.extra_turns.push_back(ACTOR);
    engine.state.extra_turns.push_back(ACTOR);
    let turns: Vec<_> = (0..4)
        .map(|_| next_turn(&mut engine, SUBJECT, |_| {}))
        .collect();
    assert_eq!(
        turns,
        [
            (ACTOR, SUBJECT),
            (ACTOR, SUBJECT),
            (SUBJECT, ACTOR),
            (ACTOR, SUBJECT),
        ]
    );
}

/// "The affected player is controlled during the entire turn; the effect
/// doesn't end until the beginning of the next turn" (CR 722.1): the next
/// turn, even when it is the subject's own extra turn right after the
/// controlled one.
#[test]
fn a_scheduled_control_ends_before_the_subjects_own_extra_turn() {
    let mut engine = scheduled(2, 7_223, SUBJECT);
    let controlled = next_turn(&mut engine, SUBJECT, |engine| {
        // Taken during the controlled turn, as Time Walk would be.
        engine.state.extra_turns.push_front(SUBJECT);
    });
    assert_eq!(controlled, (SUBJECT, ACTOR));
    let extra = next_turn(&mut engine, SUBJECT, |_| {});
    assert_eq!(
        extra,
        (SUBJECT, SUBJECT),
        "the extra turn is the subject's own"
    );
    assert!(engine.state.turn_controls.is_empty());
}

#[test]
fn finishing_a_constrained_payment_cannot_discard_an_unconsumed_obligation() {
    let mut engine = at_controlled_target();
    let name = engine.state.names.intern("Public mana source");
    let land = engine.state.create_bare(
        SUBJECT,
        crate::object::ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let object = engine.state.object_mut(land).unwrap();
    object.base_mut().types = TypeSet::LAND;
    object
        .base_mut()
        .subtypes
        .insert(baylee_core::generated::subtypes::land::ISLAND);
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();
    engine
        .apply(ACTOR, PlayerAction::ChoosePlayer(SUBJECT))
        .unwrap();
    assert!(engine.mana_window.is_some());

    // Exercise the completion invariant independently of the offer planner:
    // two obligated units cannot disappear when the selected cost is only one.
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    engine
        .state
        .constrained_payments
        .last_mut()
        .unwrap()
        .required
        .add(ManaColor::Blue, 2);
    let before = engine.fingerprint();
    assert!(engine.apply(ACTOR, PlayerAction::PassPriority).is_err());
    assert_eq!(engine.fingerprint(), before);
    assert_eq!(
        engine
            .state
            .constrained_payments
            .last()
            .unwrap()
            .required
            .total(),
        2
    );
}

#[test]
fn announcement_context_obeys_resource_owner_and_controller_entitlements() {
    let engine = at_controlled_target();
    let owner = engine
        .announced_spell_context(SUBJECT)
        .expect("owner sees its announcement");
    assert_eq!(owner.1, Some(0));
    assert_eq!(engine.announced_spell_context(ACTOR), Some(owner));
    assert_eq!(engine.announced_spell_context(PlayerId::new(2)), None);
}

#[test]
fn scheduled_control_ends_as_soon_as_either_player_leaves() {
    for departing in [ACTOR, SUBJECT] {
        let mut engine = fixture();
        engine
            .state
            .turn_controls
            .push(crate::state::PlayerTurnControl {
                controller: ACTOR,
                player: SUBJECT,
                turn: engine.state.turn.number + 1,
                began: None,
            });
        engine.apply(departing, PlayerAction::Concede).unwrap();
        assert!(engine.state.turn_controls.is_empty());
        assert_eq!(engine.controlled_actor(SUBJECT), SUBJECT);
    }
}
