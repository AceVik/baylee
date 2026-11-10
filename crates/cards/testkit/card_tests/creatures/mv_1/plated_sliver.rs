//! `cards/creatures/mv_1/plated_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plated Sliver prints one sentence — "All Sliver creatures get +0/+1" — and
/// every word of it is load-bearing. It says *all*, not "you control", so the
/// Sliver across the table is pumped too; it says *Sliver*, so the Llanowar
/// Elves beside it on both boards must stay 1/1; and it says *creatures*, so
/// the Sliver printing the sentence is on its own menu and is a 1/2 when it
/// stands alone. With one on each side the two statics are separate objects
/// and stack, which is why each Sliver reads 1/3 and neither Elf moved — a
/// filter that had dropped "Sliver" would pump the whole table, and one that
/// had quietly added "you control" would leave the far Sliver a 1/2.
#[test]
fn plated_sliver_plates_every_sliver_on_the_table_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plated_sliver(), llanowar_elves()])
        .battlefield(1, &[plated_sliver(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, plated_sliver()).expect("p0's Sliver is out");
    let theirs = on_battlefield(&engine, p1, plated_sliver()).expect("p1's Sliver is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0's Elves are out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elves are out");

    assert_eq!(
        pt(&engine, mine),
        (1, 3),
        "a printed 1/1 taking +0/+1 from its own sentence and +0/+1 again \
         from the one across the table: the static is a Sliver creature's own \
         and it stacks per instance"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 3),
        "\"all Sliver creatures\" is not \"Slivers you control\" — the far \
         Sliver is plated by both statics exactly the same way"
    );
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "an Elf is no Sliver and gets nothing"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "and the filter is the same one on both sides of the table"
    );
}
