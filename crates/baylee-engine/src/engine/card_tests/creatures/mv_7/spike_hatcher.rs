//! `cards/creatures/mv_7/spike_hatcher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spike_hatcher() -> CardIndex {
    card_index("726fe7a4-49b3-4b67-9d8c-6b334d0fd915")
}

/// Spike Hatcher cast off seven of ten Forests, three left over for its
/// abilities, with a Gray Ogre out as a creature to point at.
#[track_caller]
fn a_hatcher_with_three_forests_over() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let board = [lots(forest(), 10), vec![gray_ogre()]].concat();
    let mut engine = table7(&board, &[spike_hatcher()], &[]);
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| forests[..7].contains(&id));
    cast_with_floating(&mut engine, p0, spike_hatcher());
    pass_until(&mut engine, stack_is_empty);
    let hatcher = on_battlefield(&engine, p0, spike_hatcher()).expect("resolved");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    (engine, hatcher, ogre)
}

/// "This creature enters with six +1/+1 counters on it": a 0/0 body that is
/// 6/6 by the counters.
#[test]
fn spike_hatcher_enters_with_six_counters() {
    let (engine, hatcher, _ogre) = a_hatcher_with_three_forests_over();
    assert_eq!(counters_on(&engine, hatcher, CounterKind::P1P1), 6);
    assert_eq!(pt(&engine, hatcher), (6, 6));
}

/// "{2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on
/// target creature." The Ogre gets it, the Hatcher loses it.
#[test]
fn spike_hatcher_moves_a_counter_to_another_creature() {
    let p0 = PlayerId::new(0);
    let (mut engine, hatcher, ogre) = a_hatcher_with_three_forests_over();
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| forests[7..].contains(&id));
    activate(&mut engine, p0, spike_hatcher(), 0);
    let _ = aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, hatcher, CounterKind::P1P1), 5);
    assert_eq!(counters_on(&engine, ogre, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, ogre), (3, 3));
    assert_eq!(pt(&engine, hatcher), (5, 5));
}

/// "{1}, Remove a +1/+1 counter from this creature: Regenerate this
/// creature." The shield stands, the counter is the price, and destruction
/// regenerates it with five counters.
#[test]
fn spike_hatcher_regenerates_for_a_counter() {
    let p0 = PlayerId::new(0);
    let (mut engine, hatcher, _ogre) = a_hatcher_with_three_forests_over();
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| forests[7..].contains(&id));
    raise_a_shield(&mut engine, p0, hatcher, 1);
    assert_eq!(
        counters_on(&engine, hatcher, CounterKind::P1P1),
        5,
        "the price"
    );
    kill(&mut engine, hatcher);
    assert_eq!(on_battlefield(&engine, p0, spike_hatcher()), Some(hatcher));
}
