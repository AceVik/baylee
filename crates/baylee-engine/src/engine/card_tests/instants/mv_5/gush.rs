//! `cards/instants/mv_5/gush.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "16d8ee99-8ec8-429f-9ba7-818a74b6f910"
fn gush() -> CardIndex {
    card_index("16d8ee99-8ec8-429f-9ba7-818a74b6f910")
}

/// Gush — {4}{U} Instant: "Draw two cards." (`Coverage::Partial`: returning two
/// Islands alternative cost is not supported).
#[test]
fn gush_draws_two_cards() {
    let mut engine = table(
        SEED,
        &[gush()],
        &[island(), island(), island(), island(), island()],
        &[],
        &[],
    );
    let before = hand_len(&engine, me());
    cast_plain(&mut engine, gush());
    assert_eq!(hand_len(&engine, me()), before - 1 + 2);
}
