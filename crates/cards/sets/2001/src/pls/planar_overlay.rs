//! Planar Overlay — {2}{U} — Sorcery
//! Oracle: Each player chooses a land they control of each basic land type. Return those lands to their owners' hands.
//! Set: PLS #28 — Planeshift | Scryfall ID: 1315fef0-234e-44f5-a7a3-bf3db78943c3 | Oracle ID: a7be1624-04cc-4ed5-8b05-63b633893dee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_OVERLAY,
    oracle_id = "a7be1624-04cc-4ed5-8b05-63b633893dee",
    scryfall_id = "1315fef0-234e-44f5-a7a3-bf3db78943c3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Planar Overlay",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
