//! Lórien Revealed — {3}{U}{U} — Sorcery
//! Oracle: Draw three cards.
//! Oracle: Islandcycling {1} ({1}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)
//! Set: HOC #179 — The Hobbit Eternal | Scryfall ID: 209204c7-aae5-4768-91b8-89c8c4d4a97f | Oracle ID: 66f28905-c7fc-4ada-8fa0-199626d9bedb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LORIEN_REVEALED,
    oracle_id = "66f28905-c7fc-4ada-8fa0-199626d9bedb",
    scryfall_id = "209204c7-aae5-4768-91b8-89c8c4d4a97f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lórien Revealed",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
