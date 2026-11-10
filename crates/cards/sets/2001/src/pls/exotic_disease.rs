//! Exotic Disease — {4}{B} — Sorcery
//! Oracle: Domain — Target player loses X life and you gain X life, where X is the number of basic land types among lands you control.
//! Set: PLS #43 — Planeshift | Scryfall ID: 4e9624e5-79a2-41de-997b-12d871d4be66 | Oracle ID: 9480e142-289a-439d-b6a5-2666fb0e5be3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXOTIC_DISEASE,
    oracle_id = "9480e142-289a-439d-b6a5-2666fb0e5be3",
    scryfall_id = "4e9624e5-79a2-41de-997b-12d871d4be66",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Exotic Disease",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
