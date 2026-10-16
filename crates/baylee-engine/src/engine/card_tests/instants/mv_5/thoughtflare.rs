//! `cards/instants/mv_5/thoughtflare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "faad0f02-da5b-47f4-9634-0ea6ee2ff4f6"
fn thoughtflare() -> CardIndex {
    card_index("faad0f02-da5b-47f4-9634-0ea6ee2ff4f6")
}

/// Thoughtflare: "Draw four cards, then discard two cards."
#[test]
fn thoughtflare_draws_four_then_discards_two() {
    let lands = [vec![island(); 4], vec![mountain()]].concat();
    let mut engine = table(932, &[thoughtflare()], &lands, &[], &[]);
    let library_before = library_len(&engine, me());
    cast_from_hand(&mut engine, me(), thoughtflare());
    let asked = settle_asked(&mut engine);
    let discards: Vec<_> = asked
        .iter()
        .filter(|q| matches!(q, Pending::ChooseCards { .. }))
        .collect();
    assert!(!discards.is_empty(), "a discard question: {asked:?}");
    assert_eq!(library_len(&engine, me()), library_before - 4, "four drawn");
    assert_eq!(hand_len(&engine, me()), 2, "four drawn, two discarded");
    assert_eq!(
        graveyard_len(&engine, me()),
        3,
        "the spell and two discards"
    );
}
