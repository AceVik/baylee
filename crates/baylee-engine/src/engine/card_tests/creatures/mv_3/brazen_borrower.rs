//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn borrower_blocks_flying_but_not_reach_or_ground_creatures() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::BRAZEN_BORROWER])
        .battlefield(1, &[index::ORNITHOPTER, grizzly_bears(), giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let b = on_battlefield(&e, p0, index::BRAZEN_BORROWER).unwrap();
    for (card, allowed) in [
        (index::ORNITHOPTER, true),
        (grizzly_bears(), false),
        (giant_spider(), false),
    ] {
        let a = on_battlefield(&e, p1, card).unwrap();
        assert_eq!(crate::combat::can_block(e.state(), p0, b, a), allowed);
    }
    assert!(keywords(&e, b).contains(KeywordSet::FLASH.union(KeywordSet::FLYING)));
}
