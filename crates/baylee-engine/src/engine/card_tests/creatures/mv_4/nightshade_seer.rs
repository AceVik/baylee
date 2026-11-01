//! `cards/creatures/mv_4/nightshade_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn nightshade_seer() -> CardIndex {
    card_index("9bcf66d0-8a4d-4c57-b4de-4cc35399465e")
}

/// Nightshade Seer (9bcf66d0-8a4d-4c57-b4de-4cc35399465e) — Partial: casts as
/// a 1/1 black Human Wizard for {3}{B}; the variable hand-reveal activated
/// ability is omitted and not offered.
#[test]
fn nightshade_seer_casts_as_one_one_and_does_not_offer_unimplemented_reveal_ability() {
    let mut e = lk::game(SEED, &[], &[], &[nightshade_seer(), swamp()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e, lk::P0, nightshade_seer());
    lk::settle(&mut e);

    let seer = lk::obj(&e, lk::P0, nightshade_seer());
    assert_eq!(pt(&e, seer), (1, 1));
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 3)]);
    assert!(!lk::offered(&e, nightshade_seer(), lk::P0, 0));
}
