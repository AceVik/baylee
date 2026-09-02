//! Soured Springs — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {U} or {B}.
//! Set: OTJ #264 — Outlaws of Thunder Junction | Scryfall ID: 67daa31c-d9c4-4c22-b29c-1b8a17d577e5 | Oracle ID: e579edd7-4f6c-4f22-a72f-0a20d7a698a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1068,
    oracle_id: "e579edd7-4f6c-4f22-a72f-0a20d7a698a2",
    scryfall_id: "67daa31c-d9c4-4c22-b29c-1b8a17d577e5",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces: &[
    face! {
        name: "Soured Springs",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
