//! `cards/creatures/mv_4/jasmine_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn jasmine_seer() -> CardIndex {
    card_index("d934a7f0-eb46-4a2c-b0fd-e3071e23da4a")
}

/// Jasmine Seer (d934a7f0-eb46-4a2c-b0fd-e3071e23da4a) — Partial: casts as a
/// 1/1 white Human Wizard for {3}{W}; the variable hand-reveal activated
/// ability is omitted and not offered.
#[test]
fn jasmine_seer_casts_as_one_one_and_does_not_offer_unimplemented_reveal_ability() {
    let mut e = lk::game(SEED, &[], &[], &[jasmine_seer(), plains()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut e, lk::P0, jasmine_seer());
    lk::settle(&mut e);

    let seer = lk::obj(&e, lk::P0, jasmine_seer());
    assert_eq!(pt(&e, seer), (1, 1));
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 3)]);
    assert!(!lk::offered(&e, jasmine_seer(), lk::P0, 0));
}
