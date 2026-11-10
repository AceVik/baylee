//! Syphon Mind — {3}{B} — Sorcery
//! Oracle: Each other player discards a card. You draw a card for each card discarded this way.
//! Set: MSC #159 — Marvel Super Heroes Commander | Scryfall ID: be3f418b-25c3-4370-a1de-50e8e88c2ef6 | Oracle ID: abc37d6c-6300-47b5-a679-9db5b83eb54f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYPHON_MIND,
    oracle_id = "abc37d6c-6300-47b5-a679-9db5b83eb54f",
    scryfall_id = "be3f418b-25c3-4370-a1de-50e8e88c2ef6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Syphon Mind",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
