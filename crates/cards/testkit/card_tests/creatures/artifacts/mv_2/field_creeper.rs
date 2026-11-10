//! `cards/creatures/artifacts/mv_2/field_creeper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn field_creeper() -> CardIndex {
    card_index("e6584e39-099c-437f-9239-fa718cac85af")
}

/// Field Creeper prints no rules text: a `{2}` 2/1 artifact creature.
#[test]
fn field_creeper_is_a_two_one_artifact_creature_for_two() {
    let (engine, creeper) = vanilla_cast(field_creeper(), &[forest(), forest()], (2, 1));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, creeper));
}

/// Attacking unblocked deals its two.
#[test]
fn field_creeper_attacks_for_two() {
    vanilla_swing(field_creeper(), 2);
}
