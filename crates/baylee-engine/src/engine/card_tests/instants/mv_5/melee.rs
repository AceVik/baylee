//! `cards/instants/mv_5/melee.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn melee() -> CardIndex {
    card_index("93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e")
}

/// Melee (93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e) — {4}{R} Instant:
/// Coverage::Partial — resolves with no effect.
#[test]
fn melee_casts_for_four_and_a_red_and_resolves_to_graveyard() {
    let mut engine = table(
        SEED,
        &[melee()],
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[],
        &[],
    );
    cast_plain(&mut engine, melee());
    assert!(in_graveyard(&engine, me(), melee()).is_some());
}
