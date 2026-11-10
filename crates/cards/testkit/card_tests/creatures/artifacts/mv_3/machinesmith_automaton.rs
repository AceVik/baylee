//! `cards/creatures/artifacts/mv_3/machinesmith_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn machinesmith_automaton() -> CardIndex {
    card_index("945e8443-9647-4eab-b091-ee8e7bc347f5")
}

/// Machinesmith Automaton — trample 2/2; "Whenever another artifact you
/// control enters, put a +1/+1 counter on this creature." It gets none for a
/// land or for itself, one for another artifact.
#[test]
fn machinesmith_automaton_grows_for_another_artifact_only() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[machinesmith_automaton(), copper_myr(), forest()],
        &[mountain(), forest(), forest(), forest(), forest()],
    );
    cast_from_hand(&mut engine, p0, machinesmith_automaton());
    pass_until(&mut engine, stack_is_empty);
    let auto = on_battlefield(&engine, p0, machinesmith_automaton()).expect("it resolved");
    assert!(keywords(&engine, auto).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        counters_on(&engine, auto, CounterKind::P1P1),
        0,
        "not for itself"
    );
    assert_eq!(pt(&engine, auto), (2, 2));

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, auto, CounterKind::P1P1),
        0,
        "not for a land"
    );

    cast_with_floating(&mut engine, p0, copper_myr());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, copper_myr()).is_some());
    assert_eq!(counters_on(&engine, auto, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, auto), (3, 3));
}
