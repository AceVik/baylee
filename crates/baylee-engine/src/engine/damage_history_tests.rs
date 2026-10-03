//! Actual damage, lookback and incarnation regression tests for death triggers.
use super::testkit::{
    Duel, RegistryLookup, basic_forest, keep_mulligans, on_battlefield, reach_main_phase,
};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, Amount, CounterKind, Effect, Filter, Modifier, TargetSpec, Trigger, triggered,
};
use baylee_core::generated::index;
use baylee_core::types::TypeSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const COUNTER: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];
const DEATH: &[AbilityDef] = &[triggered!(
    Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
    COUNTER
)];
const EVENT_DAMAGE: &[AbilityDef] = &[triggered!(
    Trigger::EntersBattlefield(&Filter::CREATURE),
    &[Effect::EventObjectDealsDamageEqualToPower {
        target: TargetSpec::Object(&Filter::CREATURE)
    }],
    targets = Some(baylee_cards_dsl::TargetReq::one(TargetSpec::Object(
        &Filter::CREATURE
    )))
)];

fn setup() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(7731, basic_forest())
        .battlefield(0, &[index::SENGIR_VAMPIRE])
        .battlefield(1, &[index::WALL_OF_SWORDS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let source = on_battlefield(&e, P0, index::SENGIR_VAMPIRE).unwrap();
    let victim = on_battlefield(&e, P1, index::WALL_OF_SWORDS).unwrap();
    (e, source, victim)
}

fn execute(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    on_stack: ObjectId,
    victim: ObjectId,
    event_object: Option<ObjectId>,
    effect: Effect,
) {
    let mut res = crate::resolve::Resolution {
        source,
        on_stack,
        controller: P0,
        effects: vec![effect],
        pc: 0,
        targets: smallvec::smallvec![victim],
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
    };
    assert!(matches!(
        crate::resolve::run(&mut e.state, &mut res),
        crate::resolve::Flow::Complete
    ));
}

fn damage(e: &mut Engine<RegistryLookup>, source: ObjectId, victim: ObjectId, amount: u32) {
    execute(
        e,
        source,
        source,
        victim,
        None,
        Effect::DealDamage {
            amount: Amount::Fixed(amount),
            target: TargetSpec::Object(&Filter::CREATURE),
        },
    );
}

fn move_to(e: &mut Engine<RegistryLookup>, id: ObjectId, to: ZoneLocation) {
    e.state
        .move_object(
            id,
            to,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.state.refresh_characteristics();
}

fn blink(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    let owner = e.state.object(id).unwrap().owner;
    move_to(e, id, ZoneLocation::Exile(owner));
    move_to(e, id, ZoneLocation::Battlefield);
}

fn deaths(
    e: &Engine<RegistryLookup>,
    from: u64,
    source: ObjectId,
) -> Vec<crate::trigger::PendingTrigger> {
    crate::trigger::collect(&e.state, &e.lookup, from)
        .into_iter()
        .filter(|t| t.source == source)
        .collect()
}

#[test]
fn damage_history_deduplicates_hits_and_preserves_controller_changes() {
    let (mut e, source, victim) = setup();
    let from = e.state.journal.last_seq();
    damage(&mut e, source, victim, 1);
    damage(&mut e, source, victim, 1);
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(victim),
        controller: P1,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp: 1,
        filter: crate::effects::EffectFilter::object(&e.state, source),
        modifier: Modifier::GainControl,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
    });
    e.state.refresh_characteristics();
    crate::sba::put_into_graveyard(&mut e.state, victim);
    let found = deaths(&e, from, source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].controller, P1);
}

#[test]
fn damage_history_distinguishes_both_incarnations_and_absent_sources() {
    for change in 0..4 {
        let (mut e, source, victim) = setup();
        let from = e.state.journal.last_seq();
        damage(&mut e, source, victim, 1);
        match change {
            0 => blink(&mut e, source),
            1 => blink(&mut e, victim),
            2 => e.state.phase_out(&[source]),
            _ => move_to(&mut e, source, ZoneLocation::Hand(P0)),
        }
        crate::sba::put_into_graveyard(&mut e.state, victim);
        assert!(deaths(&e, from, source).is_empty(), "case {change}");
    }
}

#[test]
fn damage_history_is_about_the_creature_at_death_not_at_damage() {
    for creature_at_damage in [false, true] {
        let (mut e, source, victim) = setup();
        let from = e.state.journal.last_seq();
        e.state.object_mut(victim).unwrap().base_mut().types = if creature_at_damage {
            TypeSet::CREATURE
        } else {
            TypeSet::ARTIFACT
        };
        e.state.invalidate_projections();
        e.state.refresh_characteristics();
        damage(&mut e, source, victim, 1);
        e.state.object_mut(victim).unwrap().base_mut().types = if creature_at_damage {
            TypeSet::ARTIFACT
        } else {
            TypeSet::CREATURE
        };
        e.state.invalidate_projections();
        e.state.refresh_characteristics();
        crate::sba::put_into_graveyard(&mut e.state, victim);
        assert_eq!(
            deaths(&e, from, source).len(),
            usize::from(!creature_at_damage)
        );
    }
}

#[test]
fn damage_history_remembers_abilities_at_death_even_if_both_objects_change_later() {
    let (mut e, source, victim) = setup();
    let from = e.state.journal.last_seq();
    e.state.object_mut(source).unwrap().own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    damage(&mut e, source, victim, 1);
    e.state.object_mut(source).unwrap().own_abilities =
        Some(crate::object::AbilityList::from_static(DEATH, None, None).into_bundle());
    crate::sba::put_into_graveyard(&mut e.state, victim);
    // A creature can leave its graveyard during the same resolution, and
    // the damage source can stop being a copy before the trigger is scanned.
    move_to(&mut e, victim, ZoneLocation::Exile(P1));
    e.state.object_mut(source).unwrap().own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    let found = deaths(&e, from, source);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .abilities
            .as_ref()
            .unwrap()
            .abilities
            .iter()
            .eq(DEATH.iter())
    );
}

#[test]
fn damage_history_does_not_trigger_when_the_ability_was_removed_before_death() {
    let (mut e, source, victim) = setup();
    let from = e.state.journal.last_seq();
    damage(&mut e, source, victim, 1);
    e.state.object_mut(source).unwrap().own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    crate::sba::put_into_graveyard(&mut e.state, victim);
    assert!(deaths(&e, from, source).is_empty());
}

#[test]
fn damage_history_separates_sequential_and_simultaneous_deaths() {
    for simultaneous in [false, true] {
        let (mut e, source, victim) = setup();
        let from = e.state.journal.last_seq();
        damage(&mut e, source, victim, 1);
        if simultaneous {
            crate::sba::destroy_all(&mut e.state, &[source, victim], false);
        } else {
            crate::sba::destroy(&mut e.state, source);
            crate::sba::destroy(&mut e.state, victim);
        }
        assert_eq!(deaths(&e, from, source).len(), usize::from(simultaneous));
    }
}

#[test]
fn damage_history_survives_cleanup_but_not_the_next_turn() {
    for next_turn in [false, true] {
        let (mut e, source, victim) = setup();
        let from = e.state.journal.last_seq();
        damage(&mut e, source, victim, 1);
        e.cleanup_ends_the_turns_effects();
        assert_eq!(e.state.object(victim).unwrap().damage, 0);
        if next_turn {
            e.state.per_turn.reset();
        }
        // A toughness-zero SBA after the cleanup removes a temporary pump
        // still sees damage dealt earlier in this same turn (CR 514.2).
        e.state.object_mut(victim).unwrap().base_mut().toughness = Some(0);
        e.state.invalidate_projections();
        e.state.refresh_characteristics();
        crate::sba::run(&mut e.state, &e.lookup);
        assert_eq!(deaths(&e, from, source).len(), usize::from(!next_turn));
    }
}

#[test]
fn damage_history_source_blink_or_phasing_after_trigger_prevents_self_counter() {
    for phased in [false, true] {
        let (mut e, source, victim) = setup();
        damage(&mut e, source, victim, 1);
        crate::sba::put_into_graveyard(&mut e.state, victim);
        e.collect_triggers();
        assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 1);
        if phased {
            e.state.phase_out(&[source]);
        } else {
            blink(&mut e, source);
        }
        e.resolve_stack_top();
        assert_eq!(
            e.state
                .object(source)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
    }
}

#[test]
fn damage_history_does_not_credit_a_returned_source_for_its_old_stacked_ability() {
    let (mut e, source, victim) = setup();
    let from = e.state.journal.last_seq();
    let on_stack = e.push_ability_to_stack(P0, source, 0, smallvec::smallvec![victim]);
    blink(&mut e, source);
    execute(
        &mut e,
        source,
        on_stack,
        victim,
        None,
        Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        },
    );
    assert_eq!(e.state.object(victim).unwrap().damage, 1);
    crate::sba::put_into_graveyard(&mut e.state, victim);
    assert!(deaths(&e, from, source).is_empty());
}

#[test]
fn damage_history_hashes_live_history_and_ignores_stale_history_for_loops() {
    let (mut e, source, victim) = setup();
    damage(&mut e, source, victim, 1);
    e.state.object_mut(victim).unwrap().damage = 0;
    let mut without = e.state.clone();
    without.per_turn.permanent_damage.clear();
    assert_ne!(e.state.snapshot_hash(), without.snapshot_hash());
    assert_ne!(e.state.loop_signature(), without.loop_signature());
    blink(&mut e, source);
    without = e.state.clone();
    without.per_turn.permanent_damage.clear();
    assert_eq!(e.state.loop_signature(), without.loop_signature());
}

#[test]
fn damage_history_granted_trigger_retains_source_identity() {
    let (mut e, source, victim) = setup();
    e.state.object_mut(source).unwrap().own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(source),
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp: 1,
        filter: crate::effects::EffectFilter::object(&e.state, source),
        modifier: Modifier::GrantTriggered {
            trigger: Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
            effects: COUNTER,
            target: None,
        },
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
    });
    e.state.refresh_characteristics();
    damage(&mut e, source, victim, 1);
    crate::sba::put_into_graveyard(&mut e.state, victim);
    e.collect_triggers();
    assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 1);
    blink(&mut e, source);
    e.resolve_stack_top();
    assert_eq!(
        e.state
            .object(source)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
}

#[test]
fn event_dealer_uses_original_incarnation_and_departure_power_after_blink() {
    for blinked in [false, true] {
        let (mut e, source, victim) = setup();
        // An observer's ETB instruction makes the entering creature deal
        // damage; the damage source is the event object, not the observer.
        e.state.object_mut(victim).unwrap().own_abilities =
            Some(crate::object::AbilityList::from_static(EVENT_DAMAGE, None, None).into_bundle());
        let from = e.state.journal.last_seq();
        blink(&mut e, source);
        let found = crate::trigger::collect(&e.state, &e.lookup, from);
        let t = found.into_iter().find(|t| t.source == victim).unwrap();
        let stack = e.push_ability_to_stack(P1, victim, 0, smallvec::smallvec![victim]);
        e.bind_top_trigger(&t);
        e.state.object_mut(victim).unwrap().own_abilities =
            Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
        e.state.object_mut(source).unwrap().base_mut().power = Some(3);
        e.state.invalidate_projections();
        e.state.refresh_characteristics();
        if blinked {
            blink(&mut e, source);
            e.state.object_mut(source).unwrap().base_mut().power = Some(1);
            e.state.invalidate_projections();
            e.state.refresh_characteristics();
        }
        execute(
            &mut e,
            victim,
            stack,
            victim,
            Some(source),
            Effect::EventObjectDealsDamageEqualToPower {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
        );
        assert_eq!(
            e.state.object(victim).unwrap().damage,
            3,
            "departure LKI must beat returned object's power"
        );
        crate::sba::put_into_graveyard(&mut e.state, victim);
        assert_eq!(deaths(&e, from, source).len(), usize::from(!blinked));
    }
}
