//! Cabal Ritual — {1}{B} — Instant
//! Oracle: Add {B}{B}{B}.
//! Oracle: Threshold — Add {B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard.
//! Set: VMA #106 — Vintage Masters | Scryfall ID: a5d85875-22da-4054-ae42-e85b472a6d5d | Oracle ID: 5b5bf1fa-6502-4790-b66b-f0f8504ebc7c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_RITUAL,
    oracle_id = "5b5bf1fa-6502-4790-b66b-f0f8504ebc7c",
    scryfall_id = "a5d85875-22da-4054-ae42-e85b472a6d5d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Ritual",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
