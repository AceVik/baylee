//! Volatile Fault — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.
//! Set: LCI #286 — The Lost Caverns of Ixalan | Scryfall ID: 9385abf3-b067-4586-bf3d-175526cf8f0a | Oracle ID: 95c44f28-f7fa-4785-83b9-0d81be0db0c8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1298,
    oracle_id: "95c44f28-f7fa-4785-83b9-0d81be0db0c8",
    scryfall_id: "9385abf3-b067-4586-bf3d-175526cf8f0a",
    faces: &[
    face! {
        name: "Volatile Fault",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
