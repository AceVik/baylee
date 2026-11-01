//! Reviewed enraging licid behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn enraging_licid() -> CardIndex {
    card_index("5ec9f317-887a-4e08-a165-e7cf15eb071d")
}

#[test]
fn enraging_licid_body_casts_without_the_unprinted_tap_ability() {
    let mut e = lk::game(SEED, &[], &[], &[enraging_licid()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut e, lk::P0, enraging_licid(), &[], &[]);
    assert!(on_battlefield(&e, lk::P0, enraging_licid()).is_some());
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    assert!(!lk::offered(&e, enraging_licid(), lk::P0, 0));
}
