//! `cards/creatures/artifacts/mv_2/alpha_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn alpha_myr() -> CardIndex {
    card_index("4caa8e68-e599-416f-b839-3d009569ff29")
}

/// Alpha Myr prints no rules text: a `{2}` 2/1 artifact creature. Cast off two
/// Forests it becomes exactly that, with nothing on the stack.
#[test]
fn alpha_myr_is_a_two_one_artifact_creature_for_two() {
    let (engine, myr) = vanilla_cast(alpha_myr(), &[forest(), forest()], (2, 1));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two mana were the price"
    );
    assert!(!is_tapped(&engine, myr));
}

/// With no ability to tap for, attacking is all it does: two damage.
#[test]
fn alpha_myr_attacks_for_two() {
    vanilla_swing(alpha_myr(), 2);
}
