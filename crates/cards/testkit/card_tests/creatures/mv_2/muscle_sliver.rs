//! `cards/creatures/mv_2/muscle_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Muscle Sliver prints one line and nothing else: "All Sliver creatures get
/// +1/+1." The load-bearing word is *All*, and it names no controller, so the
/// reading that matters is the one taken across the table.
///
/// Four numbers stand on one board and three of them are refusals. The Sliver
/// itself at (2, 2) proves it counts itself; the Queen beside it at (8, 8)
/// proves "every other Sliver of mine" — a printed 7/7 plus one; the Queen
/// opposite at (8, 8) proves the static is no more yours than hers, which a
/// "Slivers you control" misreading would leave at its printed 7/7; and the Elf
/// at (1, 1) proves the filter is a subtype and not every creature on the
/// table, which a static that had lost its `HasSubtype` arm would have pumped.
#[test]
fn muscle_sliver_pumps_every_sliver_on_the_table_including_itself_and_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[muscle_sliver(), sliver_queen(), llanowar_elves()])
        .battlefield(1, &[sliver_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sliver = on_battlefield(&engine, p0, muscle_sliver()).expect("the Muscle Sliver is out");
    let mine = on_battlefield(&engine, p0, sliver_queen()).expect("my Queen is out");
    let theirs = on_battlefield(&engine, p1, sliver_queen()).expect("their Queen is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");

    assert_eq!(
        pt(&engine, sliver),
        (2, 2),
        "the printed 1/1 counts itself: a Sliver is a Sliver"
    );
    assert_eq!(
        pt(&engine, mine),
        (8, 8),
        "and the printed 7/7 beside it, so the anthem reaches every other \
         Sliver under the same seat"
    );
    assert_eq!(
        pt(&engine, theirs),
        (8, 8),
        "\"All Sliver creatures\" names no controller, so the Queen across the \
         table is pumped too — a static reading \"Slivers you control\" would \
         leave her at 7/7"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the Elf is a creature and no Sliver, so the subtype is read and not \
         skipped"
    );
}
