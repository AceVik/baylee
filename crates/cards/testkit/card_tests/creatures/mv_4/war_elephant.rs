//! `cards/creatures/mv_4/war_elephant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// War Elephant — `{3}{W}` 2/2 Elephant: "Trample; banding (…)". Both words
/// are printed on the card and nothing else on this board grants either, so
/// the projected keyword set is the whole assertion (CR 702.19b, CR 702.22).
#[test]
fn war_elephant_prints_trample_and_banding() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[war_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elephant = on_battlefield(&engine, p0, war_elephant()).expect("seated");
    assert_eq!(pt(&engine, elephant), (2, 2), "the printed body");
    let kw = keywords(&engine, elephant);
    assert!(
        kw.contains(KeywordSet::TRAMPLE),
        "trample (CR 702.19b): {kw:?}"
    );
    assert!(
        kw.contains(KeywordSet::BANDING),
        "banding (CR 702.22): {kw:?}"
    );
}
