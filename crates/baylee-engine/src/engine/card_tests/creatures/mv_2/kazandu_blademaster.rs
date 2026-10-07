//! `cards/creatures/mv_2/kazandu_blademaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kazandu Blademaster: "Whenever this creature or another Ally you control
/// enters, you may put a +1/+1 counter on this creature." The first Blademaster
/// grows as itself enters and again when the second Ally arrives; the second
/// grows only as itself.
#[test]
fn kazandu_blademaster_grows_when_it_or_another_ally_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4601, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[kazandu_blademaster(), kazandu_blademaster()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_mana_where(&mut engine, p0, |_| true);
    cast_with_floating(&mut engine, p0, kazandu_blademaster());
    pass_until(&mut engine, stack_is_empty);
    let first = on_battlefield(&engine, p0, kazandu_blademaster()).expect("the first");
    assert_eq!(
        counters_on(&engine, first, CounterKind::P1P1),
        1,
        "it entered, and it is an Ally itself"
    );
    assert_eq!(pt(&engine, first), (2, 2));

    cast_with_floating(&mut engine, p0, kazandu_blademaster());
    pass_until(&mut engine, stack_is_empty);
    let all = all_on_battlefield(&engine, p0, kazandu_blademaster());
    assert_eq!(all.len(), 2);
    let second = *all.iter().find(|o| **o != first).expect("the second");
    assert_eq!(
        counters_on(&engine, first, CounterKind::P1P1),
        2,
        "another Ally entered"
    );
    assert_eq!(
        counters_on(&engine, second, CounterKind::P1P1),
        1,
        "the second one only counts its own entry"
    );
}
