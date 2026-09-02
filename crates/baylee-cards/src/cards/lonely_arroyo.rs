//! Lonely Arroyo — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {W} or {U}.
//! Set: OTJ #260 — Outlaws of Thunder Junction | Scryfall ID: 4b778b63-e5fc-4d63-a93b-4372f32cade2 | Oracle ID: 36508a8a-d1a4-400e-bfda-09436bc4d5d4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 718,
    oracle_id: "36508a8a-d1a4-400e-bfda-09436bc4d5d4",
    scryfall_id: "4b778b63-e5fc-4d63-a93b-4372f32cade2",
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Lonely Arroyo",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
