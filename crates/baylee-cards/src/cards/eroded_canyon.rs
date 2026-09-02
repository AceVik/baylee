//! Eroded Canyon — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {U} or {R}.
//! Set: OTJ #256 — Outlaws of Thunder Junction | Scryfall ID: 5c9d080f-28d7-41d6-a4e0-5b3e3a5ed770 | Oracle ID: 852c6520-d148-4923-a312-05a9af821f24
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 473,
    oracle_id: "852c6520-d148-4923-a312-05a9af821f24",
    scryfall_id: "5c9d080f-28d7-41d6-a4e0-5b3e3a5ed770",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Eroded Canyon",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
