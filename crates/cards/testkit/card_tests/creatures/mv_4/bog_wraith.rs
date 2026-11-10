//! `cards/creatures/mv_4/bog_wraith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bog Wraith — vanilla 3/3 body plus Swampwalk: unblockable while the
/// defending player controls a Swamp, and an ordinary attacker without one.
#[test]
fn bog_wraith_is_unblockable_while_the_defender_controls_a_swamp() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bog_wraith()])
        .battlefield(1, &[llanowar_elves(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    let wraith = on_battlefield(&engine, p0, bog_wraith()).expect("seated");
    assert_eq!(pt(&engine, wraith), (3, 3));
    assert!(keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));

    let blocks = attack_and_collect_blocks(&mut engine, wraith, p1);
    assert!(
        blocks.is_empty(),
        "the defender controls a Swamp, so nothing may block it: {blocks:?}"
    );
}

/// Bog Wraith without a Swamp on the other side of the table: an ordinary
/// blockable attacker, the control for the test above.
#[test]
fn bog_wraith_is_blockable_without_a_swamp_to_walk_over() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bog_wraith()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let wraith = on_battlefield(&engine, p0, bog_wraith()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, wraith, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&wraith)),
        "no Swamp: an ordinary block is legal: {blocks:?}"
    );
}
