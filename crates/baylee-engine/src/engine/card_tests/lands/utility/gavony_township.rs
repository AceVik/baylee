//! `cards/lands/utility/gavony_township.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gavony Township: "{T}: Add {C}." / "{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."
/// Paid with four basic lands, the activated ability resolves to bolster all controlled creatures.
/// Llanowar Elves receives a +1/+1 counter, increasing its power and toughness to 2/2.
#[test]
fn gavony_township_puts_plus_one_plus_one_counter_on_each_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(
            0,
            &[
                gavony_township(),
                forest(),
                forest(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let township = on_battlefield(&engine, p0, gavony_township()).expect("Township deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_mana_except(&mut engine, p0, township);
    activate(&mut engine, p0, gavony_township(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, elves), (2, 2));
    assert!(is_tapped(&engine, township));
}
