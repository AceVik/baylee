//! Promising Vein — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Set: LCI #279 — The Lost Caverns of Ixalan | Scryfall ID: e9681a54-6413-4ff4-b6b1-ee4decb25bfa | Oracle ID: 861eb7d7-7616-4620-a4fd-4b8c3bf00dd1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 879,
    oracle_id: "861eb7d7-7616-4620-a4fd-4b8c3bf00dd1",
    scryfall_id: "e9681a54-6413-4ff4-b6b1-ee4decb25bfa",
    faces: &[
    face! {
        name: "Promising Vein",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
