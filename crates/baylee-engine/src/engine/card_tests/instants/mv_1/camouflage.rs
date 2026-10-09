//! `cards/instants/mv_1/camouflage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Camouflage is written: its piles are played in
/// `engine::camouflage_tests`, and this file waits for the card's own test
/// (the stub the hook asks for). The one thing held here: it is no longer
/// the shell that cast and said nothing.
#[test]
fn camouflage_is_written() {
    let camouflage = card_index("9cf44db4-627a-4197-9588-6da72e41f03d");
    let def = baylee_cards::by_index(camouflage).expect("in the pool");
    assert_eq!(def.coverage, baylee_cards_dsl::Coverage::Implemented);
    assert!(!def.abilities.is_empty());
}
