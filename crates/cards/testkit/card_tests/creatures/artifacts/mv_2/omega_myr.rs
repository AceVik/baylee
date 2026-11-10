//! `cards/creatures/artifacts/mv_2/omega_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn omega_myr() -> CardIndex {
    card_index("fd1b237a-7132-4875-b738-8ec65b85ea28")
}

/// Omega Myr prints no rules text: a `{2}` 1/2 artifact creature.
#[test]
fn omega_myr_is_a_one_two_artifact_creature_for_two() {
    let (engine, myr) = vanilla_cast(omega_myr(), &[forest(), forest()], (1, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, myr));
}

/// Attacking unblocked deals its single point.
#[test]
fn omega_myr_attacks_for_one() {
    vanilla_swing(omega_myr(), 1);
}
