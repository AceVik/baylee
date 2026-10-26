//! `cards/creatures/artifacts/mv_3/metalworker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn metalworker() -> CardIndex {
    card_index("9ed9accb-9f0c-40fa-aeea-2d1e1c9d1bd1")
}

/// Metalworker (9ed9accb-9f0c-40fa-aeea-2d1e1c9d1bd1) — Partial: casts as a
/// 1/2 colorless Artifact Creature — Construct for {3}; the variable
/// hand-reveal mana ability is omitted and not offered.
#[test]
fn metalworker_casts_as_one_two_artifact_creature_and_does_not_offer_reveal_ability() {
    let mut e = lk::game(SEED, &[], &[], &[metalworker(), sol_ring()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Colorless, 3)]);
    cast_with_floating(&mut e, lk::P0, metalworker());
    lk::settle(&mut e);

    let worker = lk::obj(&e, lk::P0, metalworker());
    assert_eq!(pt(&e, worker), (1, 2));
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(!lk::offered(&e, metalworker(), lk::P0, 0));
}
