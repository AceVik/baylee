//! Urza's Tower — (no cost) — Land — Urza's Tower
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.
//! Set: CMM #1053 — Commander Masters | Scryfall ID: 1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752 | Oracle ID: 32fbb638-ab14-4e8b-a07a-d4c44e3496f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1271,
    oracle_id: "32fbb638-ab14-4e8b-a07a-d4c44e3496f2",
    scryfall_id: "1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752",
    faces: &[
    face! {
        name: "Urza's Tower",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::URZA_S, subtypes::land::TOWER],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
