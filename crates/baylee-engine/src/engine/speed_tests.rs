//! Speed's inherent ability uses the normal stack and copying machinery.
use super::testkit::*;
use super::*;
use baylee_cards_dsl::Effect;

fn waiting_at(speed: u8) -> (Engine<RegistryLookup>, crate::trigger::PendingTrigger) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(179, basic_forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let state = engine.dev_state_mut(p0).unwrap();
    state.players[0].speed = speed;
    let from = state.journal.last_seq();
    state.change_life(p1, -1, Cause::Effect);
    let trigger = crate::trigger::collect(state, &RegistryLookup, from)
        .into_iter()
        .find(|t| matches!(t.synthetic_effects, Some([Effect::IncreaseSpeed])))
        .expect("life loss produced speed's inherent trigger");
    pass_until(&mut engine, |e| !stack_is_empty(e));
    assert_eq!(engine.state().players[0].speed, speed);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    (engine, trigger)
}

#[test]
fn speed_trigger_waits_for_priority_and_resolves_without_a_source() {
    let (mut engine, _) = waiting_at(1);
    let id = engine.state().zones.list(ZoneLocation::Stack)[0];
    let ability = engine.state().object(id).unwrap();
    assert_eq!(engine.state().names.get(ability.base.name), "Speed");
    assert_eq!(ability.ability.unwrap().source, ObjectId::NO_SOURCE);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].speed, 2);
}

#[test]
fn speed_trigger_copy_increases_speed_but_rechecks_the_maximum() {
    static COPY: &[Effect] = &[Effect::CopyTargetAbility];
    for initial in [1, 3] {
        let (mut engine, mut copying) = waiting_at(initial);
        let original = engine.state().zones.list(ZoneLocation::Stack)[0];
        copying.synthetic_effects = Some(COPY);
        engine.push_synthetic_trigger_with_targets(&copying, smallvec::smallvec![original]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[0].speed, (initial + 2).min(4));
    }
}

#[test]
fn speed_trigger_removed_from_stack_cannot_increase_speed_or_trigger_again() {
    let (mut engine, _) = waiting_at(1);
    let id = engine.state().zones.list(ZoneLocation::Stack)[0];
    let state = engine.dev_state_mut(PlayerId::new(0)).unwrap();
    state.zones.remove(id, ZoneLocation::Stack);
    state.arena.remove(id);
    let from = state.journal.last_seq();
    state.change_life(PlayerId::new(1), -1, Cause::Effect);
    assert!(crate::trigger::collect(state, &RegistryLookup, from).is_empty());
    assert_eq!(state.players[0].speed, 1);
}
