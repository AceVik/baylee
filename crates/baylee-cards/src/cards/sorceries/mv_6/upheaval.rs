//! Upheaval — {4}{U}{U} — Sorcery
//! Oracle: Return all permanents to their owners' hands.
//! Set: MH2 #270 — Modern Horizons 2 | Scryfall ID: befe74b1-c487-42bb-a1a1-4d13f3a86ff7 | Oracle ID: 7cafc972-a6f5-4cac-a3d3-8a3ae36ffb1e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UPHEAVAL,
    oracle_id = "7cafc972-a6f5-4cac-a3d3-8a3ae36ffb1e",
    scryfall_id = "befe74b1-c487-42bb-a1a1-4d13f3a86ff7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Upheaval",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
