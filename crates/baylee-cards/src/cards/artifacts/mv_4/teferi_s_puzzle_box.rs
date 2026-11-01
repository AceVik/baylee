//! Teferi's Puzzle Box — {4} — Artifact
//! Oracle: At the beginning of each player's draw step, that player puts the cards in their hand on the bottom of their library in any order, then draws that many cards.
//! Set: 9ED #312 — Ninth Edition | Scryfall ID: 415e81e9-ca65-4bc0-8aab-d905d58fe6cc | Oracle ID: 37abcc92-9466-47ea-9e0b-5eda2eb62c8e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_PUZZLE_BOX,
    oracle_id = "37abcc92-9466-47ea-9e0b-5eda2eb62c8e",
    scryfall_id = "415e81e9-ca65-4bc0-8aab-d905d58fe6cc",
    faces = &[face!(
        name = "Teferi's Puzzle Box",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
