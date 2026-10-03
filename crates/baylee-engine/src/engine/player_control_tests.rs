//! Actor authentication and exact resource obligations are shared rules primitives.
use super::synthetic::{self, SyntheticLookup};
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
