//! Gateway Plaza — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice it unless you pay {1}.
//! Oracle: {T}: Add one mana of any color.
//! Set: WAR #246 — War of the Spark | Scryfall ID: 81d69cf2-8643-4926-857e-febbd54d870f | Oracle ID: a6543f71-0326-4e1f-b58f-9ce325d5d036
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 537,
    oracle_id: "a6543f71-0326-4e1f-b58f-9ce325d5d036",
    scryfall_id: "81d69cf2-8643-4926-857e-febbd54d870f",
    faces: &[
    face! {
        name: "Gateway Plaza",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
