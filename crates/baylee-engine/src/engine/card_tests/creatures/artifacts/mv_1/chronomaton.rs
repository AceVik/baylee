//! `cards/creatures/artifacts/mv_1/chronomaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chronomaton() -> CardIndex {
    card_index("610f25b0-770f-457a-b0eb-6deb832f71d4")
}

/// "{1}, {T}: Put a +1/+1 counter on this creature." The tap is part of the
/// price: once tapped, a second activation is not offered even with mana to
/// spare.
#[test]
fn chronomaton_grows_by_a_counter_per_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[chronomaton()], &[]);
    let me = on_battlefield(&engine, p0, chronomaton()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 1));
    assert_eq!(counters_on(&engine, me, CounterKind::P1P1), 0);

    float_mana(&mut engine, ManaColor::Colorless, 2);
    activate(&mut engine, p0, chronomaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, me, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, me), (2, 2));
    assert!(is_tapped(&engine, me), "{{T}} was part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}} was the price"
    );
    assert!(
        !is_offered(&engine, me, 0),
        "tapped, so no second activation despite the mana left"
    );
}
