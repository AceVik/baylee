//! Reviewed jet medallion behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

use baylee_core::generated::index;
fn jet_medallion() -> CardIndex {
    card_index("bccb06f1-8839-4104-9ffa-c79a817f8378")
}

#[test]
fn jet_medallion_discounts_only_your_matching_spells_and_keeps_colored_costs() {
    let mut e = lk::game(
        SEED,
        &[jet_medallion()],
        &[],
        &[index::DRUDGE_SKELETONS, index::HILL_GIANT],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Colorless, 1)]);
    assert!(!lk::castable(&e, lk::P0, index::DRUDGE_SKELETONS));
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 1)]);
    lk::cast_at(&mut e, lk::P0, index::DRUDGE_SKELETONS, &[], &[]);
    assert!(on_battlefield(&e, lk::P0, index::DRUDGE_SKELETONS).is_some());
    assert_eq!(lk::pool_total(&e, lk::P0), 1);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    assert!(!lk::castable(&e, lk::P0, index::HILL_GIANT));
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::HILL_GIANT, &[], &[]);
    assert_eq!(lk::pool_total(&e, lk::P0), 0);

    let mut e = lk::game(
        SEED,
        &[jet_medallion()],
        &[],
        &[],
        &[index::DRUDGE_SKELETONS],
    );
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Black, 1)]);
    assert!(!lk::castable(&e, lk::P1, index::DRUDGE_SKELETONS));
    lk::float(&mut e, lk::P1, &[(ManaColor::Colorless, 1)]);
    lk::cast_at(&mut e, lk::P1, index::DRUDGE_SKELETONS, &[], &[]);
    assert_eq!(lk::pool_total(&e, lk::P1), 0);
}
