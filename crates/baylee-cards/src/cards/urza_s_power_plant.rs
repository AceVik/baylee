//! Urza's Power Plant — (no cost) — Land — Urza's Power-Plant
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1052 — Commander Masters | Scryfall ID: b0449a19-37f7-4169-9e32-928db5ec76fe | Oracle ID: e11966cd-2ee3-4df4-b099-abf42dcdf0db
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1270,
    oracle_id: "e11966cd-2ee3-4df4-b099-abf42dcdf0db",
    scryfall_id: "b0449a19-37f7-4169-9e32-928db5ec76fe",
    faces: &[
    face! {
        name: "Urza's Power Plant",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::URZA_S, subtypes::land::POWER_PLANT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
