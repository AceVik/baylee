//! `cards/creatures/mv_1/shanodin_dryads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shanodin Dryads — vanilla 1/1 body plus Forestwalk: unblockable while
/// the defending player controls a Forest.
#[test]
fn shanodin_dryads_is_unblockable_while_the_defender_controls_a_forest() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[shanodin_dryads()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let dryads = on_battlefield(&engine, p0, shanodin_dryads()).expect("seated");
    assert_eq!(pt(&engine, dryads), (1, 1));
    assert!(keywords(&engine, dryads).contains(KeywordSet::FORESTWALK));

    let blocks = attack_and_collect_blocks(&mut engine, dryads, p1);
    assert!(
        blocks.is_empty(),
        "the defender controls a Forest, so nothing may block it: {blocks:?}"
    );
}

/// Shanodin Dryads without a Forest on the other side: an ordinary
/// blockable attacker, the control for the test above.
#[test]
fn shanodin_dryads_is_blockable_without_a_forest_to_walk_over() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[shanodin_dryads()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let dryads = on_battlefield(&engine, p0, shanodin_dryads()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, dryads, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&dryads)),
        "no Forest: an ordinary block is legal: {blocks:?}"
    );
}
