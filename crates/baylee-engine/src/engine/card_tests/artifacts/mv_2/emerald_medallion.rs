//! Reviewed emerald medallion behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

use baylee_core::generated::index;
fn emerald_medallion() -> CardIndex {
    card_index("d683024d-8fe7-4509-851b-7655fadc1de7")
}

#[test]
fn emerald_medallion_discounts_only_your_matching_spells_and_keeps_colored_costs() {
    let mut e = lk::game(
        SEED,
        &[emerald_medallion()],
        &[],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Colorless, 1)]);
    assert!(!lk::castable(&e, lk::P0, index::GRIZZLY_BEARS));
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut e, lk::P0, index::GRIZZLY_BEARS, &[], &[]);
    assert!(on_battlefield(&e, lk::P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::pool_total(&e, lk::P0), 1);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    assert!(!lk::castable(&e, lk::P0, index::HILL_GIANT));
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::HILL_GIANT, &[], &[]);
    assert_eq!(lk::pool_total(&e, lk::P0), 0);

    let mut e = lk::game(
        SEED,
        &[emerald_medallion()],
        &[],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    reach_their_main_phase(&mut e, lk::P1);
    lk::float(&mut e, lk::P1, &[(ManaColor::Green, 1)]);
    assert!(!lk::castable(&e, lk::P1, index::GRIZZLY_BEARS));
    lk::float(&mut e, lk::P1, &[(ManaColor::Colorless, 1)]);
    lk::cast_at(&mut e, lk::P1, index::GRIZZLY_BEARS, &[], &[]);
    assert_eq!(lk::pool_total(&e, lk::P1), 0);
}
