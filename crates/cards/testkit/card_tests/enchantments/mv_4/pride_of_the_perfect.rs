//! `cards/enchantments/mv_4/pride_of_the_perfect.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pride_of_the_perfect() -> CardIndex {
    card_index("83e2b251-bd60-4c5c-b9e2-32c22b28fb8b")
}

/// "Elves you control get +2/+0": my Elf is a 3/1, an opposing Elf and my
/// non-Elf creature are as printed.
#[test]
fn pride_of_the_perfect_pumps_only_my_elves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[pride_of_the_perfect(), llanowar_elves(), grizzly_bears()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, mine), (3, 1), "+2/+0 on my Elf");
    assert_eq!(pt(&engine, theirs), (1, 1), "an opposing Elf is not mine");
    assert_eq!(pt(&engine, bears), (2, 2), "Bears are no Elf");
}
