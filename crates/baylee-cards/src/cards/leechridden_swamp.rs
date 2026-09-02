//! Leechridden Swamp — (no cost) — Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Oracle: This land enters tapped.
//! Oracle: {B}, {T}: Each opponent loses 1 life. Activate only if you control two or more black permanents.
//! Set: DSC #286 — Duskmourn: House of Horror Commander | Scryfall ID: a07a0e31-ace6-40ad-8700-2d58135b5320 | Oracle ID: d83c86c1-126d-49e9-9b13-9e55784c49c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 708,
    oracle_id: "d83c86c1-126d-49e9-9b13-9e55784c49c5",
    scryfall_id: "a07a0e31-ace6-40ad-8700-2d58135b5320",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Leechridden Swamp",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
