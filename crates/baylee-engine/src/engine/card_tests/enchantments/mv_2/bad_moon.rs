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

/// Bad Moon names no controller: "Black creatures get +1/+1." The opponent's
/// black creature grows under *our* Bad Moon (a `you control` filter would
/// leave it at 1/1), their green creature and our own green creature stay as
/// printed, and a second black creature on our side shows the bonus is a
/// plain +1/+1 on each, not a shared one.
#[test]
fn bad_moon_pumps_the_opponents_black_creatures_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bad_moon(), llanowar_elves(), festering_goblin()])
        .battlefield(1, &[festering_goblin(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their black creature");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their green creature");
    let own_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("our green creature");
    let own_goblin = on_battlefield(&engine, p0, festering_goblin()).expect("our black creature");
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "the opponent's black creature gets +1/+1 from a Bad Moon we control"
    );
    assert_eq!(pt(&engine, own_goblin), (2, 2), "ours grows by the same");
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "their green creature: as printed"
    );
    assert_eq!(
        pt(&engine, own_elf),
        (1, 1),
        "our green creature: as printed"
    );
}
