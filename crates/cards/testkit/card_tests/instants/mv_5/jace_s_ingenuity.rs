//! `cards/instants/mv_5/jace_s_ingenuity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ce588fd3-642e-4df9-8061-6fcfc6e81270"
fn jace_s_ingenuity() -> CardIndex {
    card_index("ce588fd3-642e-4df9-8061-6fcfc6e81270")
}

/// Jace's Ingenuity: "Draw three cards."
#[test]
fn jace_s_ingenuity_draws_three_cards() {
    let mut engine = table(922, &[jace_s_ingenuity()], &[island(); 5], &[], &[]);
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_plain(&mut engine, jace_s_ingenuity());
    assert_eq!(
        hand_len(&engine, me()),
        hand_before - 1 + 3,
        "three in, the spell out"
    );
    assert_eq!(library_len(&engine, me()), library_before - 3);
    assert_eq!(hand_len(&engine, them()), 0, "the opponent draws nothing");
}
