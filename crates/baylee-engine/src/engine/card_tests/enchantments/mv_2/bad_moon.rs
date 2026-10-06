//! `cards/enchantments/mv_2/bad_moon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bad Moon: "Black creatures get +1/+1." A black creature grows and a
/// non-black creature stays exactly as printed.
#[test]
fn bad_moon_pumps_black_creatures_and_leaves_others_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bad_moon = bad_moon();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bad_moon, festering_goblin()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("black creature seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("green creature seated");
    assert_eq!(pt(&engine, goblin), (2, 2), "black creature gets +1/+1");
    assert_eq!(pt(&engine, elf), (1, 1), "non-black creature is unaffected");
}
