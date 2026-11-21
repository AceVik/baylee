//! `cards/creatures/mv_3/kavu_scout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn kavu_scout() -> CardIndex {
    card_index("2bd91ce8-1797-4dd7-8150-0d4caa7dad61")
}

/// Kavu Scout ({2}{R} 0/2 Kavu Scout):
/// "Domain — This creature gets +1/+0 for each basic land type among lands you control."
#[test]
fn kavu_scout_counts_distinct_basic_land_types_only_among_lands_you_control() {
    // Nonbasic Wasteland and opponent's basic lands contribute 0.
    let e0 = lk::game(
        SEED,
        &[kavu_scout(), index::WASTELAND],
        &[plains(), island(), index::SWAMP],
        &[],
        &[],
    );
    assert_eq!(pt(&e0, lk::obj(&e0, lk::P0, kavu_scout())), (0, 2));

    // Duplicate Mountains count once; Mountain + Forest + Island = 3 -> 3/2.
    let mut e = lk::game(
        SEED,
        &[kavu_scout(), mountain(), mountain(), forest(), island()],
        &[],
        &[],
        &[],
    );
    let scout = lk::obj(&e, lk::P0, kavu_scout());
    assert_eq!(pt(&e, scout), (3, 2));

    let _ = lk::attack(&mut e, &[scout]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);
}
