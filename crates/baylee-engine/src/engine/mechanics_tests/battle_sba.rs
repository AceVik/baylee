//! Battle SBA decisions are staged together and use exact triggered sources.
use super::*;
use baylee_cards_dsl::{Amount, CounterKind, Filter, TargetSpec, Trigger};
use baylee_core::generated::subtypes::battle::SIEGE;
use baylee_core::ids::{DamageSourceRef, Defender};
use baylee_core::types::{SubtypeSet, SupertypeSet};

const DEVICE: u32 = 8010;
const BODY: u32 = 8011;
static ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    ),
    baylee_cards_dsl::triggered!(
        Trigger::BecomesTapped(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    ),
];

fn setup(seats: [Seat; 3]) -> Bench {
    let mut e = bench(
        8010,
        vec![
            card(
                DEVICE,
                face("Battle SBA device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                ABILITIES,
            ),
            card(
                BODY,
                creature_face("Battle SBA body", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        seats,
    );
    to_main(&mut e, me());
    e
}

/// Fixture conversion isolates the SBA from entry and defeat-trigger replacement.
fn battle(
    e: &mut Bench,
    id: ObjectId,
    siege: bool,
    defense: u16,
    protector: PlayerId,
) -> DamageSourceRef {
    let object = e.state.object_mut(id).unwrap();
    let base = object.base_mut();
    base.types = TypeSet::BATTLE;
    base.subtypes = if siege {
        SubtypeSet::from_slice(&[SIEGE])
    } else {
        SubtypeSet::EMPTY
    };
    object.counters.set(CounterKind::Defense, defense);
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    let reference = e.state.source_identity(id).unwrap();
    crate::battle::set_protector(&mut e.state, reference, protector);
    reference
}

fn trigger_and_progress(e: &mut Bench, source: ObjectId) {
    e.state.set_tapped(source, true);
    e.state.journal.record(GameEvent::ObjectTapped {
        object: source,
        cause: Cause::Effect,
    });
    e.awaiting_answer = false;
    e.run_until_choice();
}

#[test]
fn any_trigger_of_the_exact_siege_holds_it_while_queued_and_stacked() {
    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    battle(&mut e, source, true, 0, them());
    trigger_and_progress(&mut e, source);
    assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    assert!(
        !e.trigger_queue.is_empty(),
        "the non-defeat trigger is still awaiting targets"
    );
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    e.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![me()],
        },
    )
    .unwrap();
    assert!(e.trigger_queue.is_empty());
    assert!(!e.state.zones.stack_is_empty());
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    settle(&mut e, me());
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
}

#[test]
fn activated_abilities_do_not_hold_zero_defense_and_non_sieges_do_not_wait_for_triggers() {
    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    activate(&mut e, me(), source, 0);
    battle(&mut e, source, true, 0, them());
    e.awaiting_answer = false;
    e.run_until_choice();
    assert!(
        !e.state.zones.stack_is_empty(),
        "the activation has not resolved"
    );
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);

    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    battle(&mut e, source, false, 0, me());
    trigger_and_progress(&mut e, source);
    assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_trigger_from_before_a_blink_does_not_hold_the_new_siege() {
    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    let old = battle(&mut e, source, true, 0, them());
    trigger_and_progress(&mut e, source);
    e.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![me()],
        },
    )
    .unwrap();
    e.state
        .move_object(
            source,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    // This fixture changed the base directly, so restore its printed artifact
    // before reentry rather than starting an unrelated protector-entry choice.
    let base = e.state.object_mut(source).unwrap().base_mut();
    base.types = TypeSet::ARTIFACT;
    base.subtypes = SubtypeSet::EMPTY;
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    e.state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    let new = battle(&mut e, source, true, 0, them());
    assert_ne!(old, new);
    e.awaiting_answer = false;
    e.run_until_choice();
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    assert!(!e.state.zones.stack_is_empty());
}

fn attack(e: &mut Bench, attacker: ObjectId, battle: ObjectId) {
    let defender = Defender::Battle(battle);
    crate::combat::remember_attack(&mut e.state, attacker, defender);
    e.state
        .combat
        .declare_attackers([crate::combat::AttackerInfo {
            creature: attacker,
            defending: defender,
            blocked: false,
            band: None,
        }]);
}

#[test]
fn missing_protector_waits_for_attackers_but_an_ineligible_existing_one_does_not() {
    let third = PlayerId::new(2);
    let mut e = setup([
        Seat::with(&[DEVICE, BODY]),
        Seat::default(),
        Seat::default(),
    ]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    let attacker = the(&e, ZoneLocation::Battlefield, BODY);
    let reference = battle(&mut e, source, true, 1, them());
    attack(&mut e, attacker, source);
    crate::sba::eliminate_player(&mut e.state, them(), crate::event::LossReason::Conceded);
    assert!(e.state.combat.is_being_attacked(reference));
    let outcome = crate::sba::run(&mut e.state, &e.lookup);
    assert!(outcome.battle_protector.is_none());
    assert_eq!(crate::battle::protector(&e.state, source), Some(them()));
    e.state.combat.remove_from_combat(attacker);
    crate::sba::run(&mut e.state, &e.lookup);
    assert_eq!(crate::battle::protector(&e.state, source), Some(third));

    let mut e = setup([
        Seat::with(&[DEVICE, BODY]),
        Seat::default(),
        Seat::default(),
    ]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    let attacker = the(&e, ZoneLocation::Battlefield, BODY);
    let reference = battle(&mut e, source, true, 1, me());
    attack(&mut e, attacker, source);
    assert!(e.state.combat.is_being_attacked(reference));
    let outcome = crate::sba::run(&mut e.state, &e.lookup);
    let (player, asked, options) = outcome
        .battle_protector
        .expect("invalid existing protector repaired during combat");
    assert_eq!((player, asked), (me(), reference));
    assert_eq!(options, vec![them(), third]);
    assert!(crate::battle::sba::answer(
        &mut e.state,
        asked,
        player,
        third
    ));
    assert_eq!(
        crate::battle::protector(&e.state, source),
        Some(me()),
        "answer stages intent only"
    );
    crate::sba::run(&mut e.state, &e.lookup);
    assert_eq!(crate::battle::protector(&e.state, source), Some(third));
    assert!(!e.state.combat.is_being_attacked(reference));
    assert!(e.state.combat.is_attacking(attacker));
}

#[test]
fn protector_and_legend_choices_follow_apnap_and_commit_with_other_sbas() {
    let third = PlayerId::new(2);
    let mut e = setup([
        Seat::with(&[DEVICE, BODY]),
        Seat::with(&[DEVICE, DEVICE]),
        Seat::default(),
    ]);
    let own = objects(&e, ZoneLocation::Battlefield, DEVICE)
        .into_iter()
        .find(|id| e.state.object(*id).unwrap().controller == me())
        .unwrap();
    let legends: Vec<_> = objects(&e, ZoneLocation::Battlefield, DEVICE)
        .into_iter()
        .filter(|id| e.state.object(*id).unwrap().controller == them())
        .collect();
    let reference = battle(&mut e, own, true, 1, me());
    for id in &legends {
        e.state.object_mut(*id).unwrap().base_mut().supertypes = SupertypeSet::LEGENDARY;
    }
    let body = the(&e, ZoneLocation::Battlefield, BODY);
    e.state.object_mut(body).unwrap().damage = 1;
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    let mut opposite_order = e.state.clone();
    opposite_order.turn.active = them();
    let opposite = crate::sba::run(&mut opposite_order, &e.lookup);
    assert!(opposite.legend_choice.is_some());
    assert!(opposite.battle_protector.is_none());
    e.awaiting_answer = false;
    e.run_until_choice();
    assert!(matches!(e.pending(), Pending::ChoosePlayer { player, .. } if *player == me()));
    let hash = e.snapshot_hash();
    assert!(e.apply(them(), PlayerAction::ChoosePlayer(third)).is_err());
    assert_eq!(e.snapshot_hash(), hash);
    e.apply(me(), PlayerAction::ChoosePlayer(third)).unwrap();
    assert!(matches!(e.pending(), Pending::LegendChoice { player, .. } if *player == them()));
    assert_eq!(crate::battle::protector(&e.state, own), Some(me()));
    assert_eq!(e.state.object(body).unwrap().zone, Zone::Battlefield);
    let saved = crate::engine::checkpoint::Checkpoint::capture(&e);
    let mut replay = setup([
        Seat::with(&[DEVICE, BODY]),
        Seat::with(&[DEVICE, DEVICE]),
        Seat::default(),
    ]);
    saved.restore(&mut replay);
    assert_eq!(e.snapshot_hash(), replay.snapshot_hash());
    for engine in [&mut e, &mut replay] {
        engine
            .apply(
                them(),
                PlayerAction::ChooseObjects {
                    objects: vec![legends[0]],
                },
            )
            .unwrap();
    }
    assert_eq!(e.snapshot_hash(), replay.snapshot_hash());
    assert_eq!(crate::battle::protector(&e.state, own), Some(third));
    assert_eq!(e.state.source_identity(own), Some(reference));
    assert_eq!(e.state.object(body).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(legends[1]).unwrap().zone, Zone::Graveyard);
}

#[test]
fn no_eligible_protector_puts_the_battle_in_the_graveyard_and_phasing_defers_zero_defense() {
    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    battle(&mut e, source, true, 1, them());
    for player in [them(), PlayerId::new(2)] {
        crate::sba::eliminate_player(&mut e.state, player, crate::event::LossReason::Conceded);
    }
    crate::sba::run(&mut e.state, &e.lookup);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);

    let mut e = setup([Seat::with(&[DEVICE]), Seat::default(), Seat::default()]);
    let source = the(&e, ZoneLocation::Battlefield, DEVICE);
    battle(&mut e, source, true, 0, them());
    e.state
        .object_mut(source)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    crate::sba::run(&mut e.state, &e.lookup);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Battlefield);
    e.state
        .object_mut(source)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    crate::sba::run(&mut e.state, &e.lookup);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
}
