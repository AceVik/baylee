//! `cards/enchantments/mv_2/crusade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crusade: "White creatures get +1/+1." A white creature grows and a
/// non-white creature stays exactly as printed.
#[test]
fn crusade_pumps_white_creatures_and_leaves_others_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let crusade = crusade();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crusade, ondu_cleric()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("white creature seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("green creature seated");
    assert_eq!(pt(&engine, cleric), (2, 2), "white creature gets +1/+1");
    assert_eq!(pt(&engine, elf), (1, 1), "non-white creature is unaffected");
}
