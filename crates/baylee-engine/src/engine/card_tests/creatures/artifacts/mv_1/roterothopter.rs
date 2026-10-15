//! `cards/creatures/artifacts/mv_1/roterothopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn roterothopter() -> CardIndex {
    card_index("69c69d69-3565-4d9c-82c0-a96d0fb80513")
}

/// "{2}: This creature gets +1/+0 until end of turn. Activate no more than
/// twice each turn." Six mana float, so mana is not what stops the third
/// activation; the next turn offers it again.
#[test]
fn roterothopter_pumps_at_most_twice_a_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[roterothopter()], &[]);
    let me = on_battlefield(&engine, p0, roterothopter()).expect("seated");
    assert_eq!(pt(&engine, me), (0, 2));
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));

    float_mana(&mut engine, ManaColor::Colorless, 6);
    activate(&mut engine, p0, roterothopter(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, me), (1, 2));
    activate(&mut engine, p0, roterothopter(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, me), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(
        !is_offered(&engine, me, 0),
        "a third time in one turn, with mana to spare"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.number >= 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, me), (0, 2), "the pump ended with its turn");
    float_mana(&mut engine, ManaColor::Colorless, 2);
    assert!(
        is_offered(&engine, me, 0),
        "a fresh turn, a fresh allowance"
    );
}
