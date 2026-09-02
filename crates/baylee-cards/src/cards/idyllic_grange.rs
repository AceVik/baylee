//! Idyllic Grange — (no cost) — Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Oracle: This land enters tapped unless you control three or more other Plains.
//! Oracle: When this land enters untapped, put a +1/+1 counter on target creature you control.
//! Set: ELD #246 — Throne of Eldraine | Scryfall ID: ca2c611c-3a6f-44b0-9daa-837a465845e0 | Oracle ID: 23d349a0-e441-40b8-b634-13e61440a7c8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 647,
    oracle_id: "23d349a0-e441-40b8-b634-13e61440a7c8",
    scryfall_id: "ca2c611c-3a6f-44b0-9daa-837a465845e0",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Idyllic Grange",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
