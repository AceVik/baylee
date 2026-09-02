//! Urza's Mine — (no cost) — Land — Urza's Mine
//! Oracle: {T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1051 — Commander Masters | Scryfall ID: 396bbb7d-ae61-4d8d-b931-9ed2f712832e | Oracle ID: 33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1269,
    oracle_id: "33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0",
    scryfall_id: "396bbb7d-ae61-4d8d-b931-9ed2f712832e",
    faces: &[
    face! {
        name: "Urza's Mine",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::URZA_S, subtypes::land::MINE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
