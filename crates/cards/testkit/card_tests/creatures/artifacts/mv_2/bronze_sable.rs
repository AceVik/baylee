//! `cards/creatures/artifacts/mv_2/bronze_sable.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bronze_sable() -> CardIndex {
    card_index("f27e5e11-0ad0-448b-8760-75ed1b97e7d8")
}

/// Bronze Sable prints no rules text: a `{2}` 2/1 artifact creature.
#[test]
fn bronze_sable_is_a_two_one_artifact_creature_for_two() {
    let (engine, sable) = vanilla_cast(bronze_sable(), &[forest(), forest()], (2, 1));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, sable));
}

/// Attacking unblocked deals its two.
#[test]
fn bronze_sable_attacks_for_two() {
    vanilla_swing(bronze_sable(), 2);
}
