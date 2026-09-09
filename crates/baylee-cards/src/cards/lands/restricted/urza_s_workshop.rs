//! Urza's Workshop — (no cost) — Land — Urza's
//! Oracle: {T}: Add {C}.
//! Oracle: Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts.
//! Set: BRC #28 — The Brothers' War Commander | Scryfall ID: 37c9b9d7-2fa7-4710-94bb-c55ee7bf598c | Oracle ID: 71099427-e110-488f-ab29-7867241fc7f0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1272,
    oracle_id: "71099427-e110-488f-ab29-7867241fc7f0",
    scryfall_id: "37c9b9d7-2fa7-4710-94bb-c55ee7bf598c",
    faces: &[
    face! {
        name: "Urza's Workshop",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::URZA_S],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
