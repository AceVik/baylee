//! Lush Portico — (no cost) — Land — Forest Plains
//! Oracle: ({T}: Add {G} or {W}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #263 — Murders at Karlov Manor | Scryfall ID: c17816e8-28b1-4295-a637-efb0e5c18873 | Oracle ID: d51831b1-7394-456e-a1de-6787a59f5932
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 729,
    oracle_id: "d51831b1-7394-456e-a1de-6787a59f5932",
    scryfall_id: "c17816e8-28b1-4295-a637-efb0e5c18873",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[
    face! {
        name: "Lush Portico",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
