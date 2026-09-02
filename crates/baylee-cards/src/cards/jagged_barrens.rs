//! Jagged Barrens — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {B} or {R}.
//! Set: OTJ #259 — Outlaws of Thunder Junction | Scryfall ID: 5d809f5b-d965-4cb1-a9f8-2048f8534373 | Oracle ID: 64ee02f1-afdb-474b-a893-31538ad7219a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 665,
    oracle_id: "64ee02f1-afdb-474b-a893-31538ad7219a",
    scryfall_id: "5d809f5b-d965-4cb1-a9f8-2048f8534373",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Jagged Barrens",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
