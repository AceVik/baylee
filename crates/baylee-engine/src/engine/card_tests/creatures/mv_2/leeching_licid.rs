//! Reviewed leeching licid behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn leeching_licid() -> CardIndex {
    card_index("bcb5af89-a304-4de0-86af-b3f4cfabafc4")
}

#[test]
fn leeching_licid_body_casts_without_the_unprinted_tap_ability() {
    let mut e = lk::game(SEED, &[], &[], &[leeching_licid()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e, lk::P0, leeching_licid(), &[], &[]);
    assert!(on_battlefield(&e, lk::P0, leeching_licid()).is_some());
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 1)]);
    assert!(!lk::offered(&e, leeching_licid(), lk::P0, 0));
}
