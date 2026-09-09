//! Sunscorched Desert — (no cost) — Land — Desert
//! Oracle: When this land enters, it deals 1 damage to target player or planeswalker.
//! Oracle: {T}: Add {C}.
//! Set: AKH #249 — Amonkhet | Scryfall ID: 405434c7-9206-45b7-af0f-d59aae294d39 | Oracle ID: 256b8c23-589e-429d-9e6e-433d55079eb4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1119,
    oracle_id: "256b8c23-589e-429d-9e6e-433d55079eb4",
    scryfall_id: "405434c7-9206-45b7-af0f-d59aae294d39",
    faces: &[
    face! {
        name: "Sunscorched Desert",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
