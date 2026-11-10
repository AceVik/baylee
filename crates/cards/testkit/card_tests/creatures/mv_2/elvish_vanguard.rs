//! `cards/creatures/mv_2/elvish_vanguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Elvish Vanguard` prints a triggered ability under `Coverage::Implemented`:
/// "Whenever another Elf enters, put a +1/+1 counter on this creature."
/// Casting `llanowar_elves` onto the battlefield triggers this ability upon the Elf's entry,
/// placing a `CounterKind::P1P1` counter on the Vanguard and increasing its `pt` to (2, 2).
#[test]
fn elvish_vanguard_grows_when_another_elf_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1616, forest())
        .battlefield(0, &[elvish_vanguard(), forest()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vanguard = on_battlefield(&engine, p0, elvish_vanguard()).expect("vanguard is seated");
    assert_eq!(pt(&engine, vanguard), (1, 1));
    assert_eq!(counters_on(&engine, vanguard, CounterKind::P1P1), 0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, vanguard, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, vanguard), (2, 2));
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
