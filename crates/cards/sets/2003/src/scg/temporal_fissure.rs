//! Temporal Fissure — {4}{U} — Sorcery
//! Oracle: Return target permanent to its owner's hand.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: DDS #12 — Duel Decks: Mind vs. Might | Scryfall ID: c9f9ece7-ee6e-4379-9d53-209f8805a72d | Oracle ID: 8a9525f4-74a6-44f0-a1b1-d57937d7783b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPORAL_FISSURE,
    oracle_id = "8a9525f4-74a6-44f0-a1b1-d57937d7783b",
    scryfall_id = "c9f9ece7-ee6e-4379-9d53-209f8805a72d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Temporal Fissure",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
