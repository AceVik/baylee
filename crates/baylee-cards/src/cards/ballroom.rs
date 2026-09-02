//! Ballroom — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #12 — Ravnica: Clue Edition | Scryfall ID: 6e982bf8-382f-4987-bc39-28e1ce290340 | Oracle ID: cb91e842-9f06-4863-a328-2cabe1bcfe27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 256,
    oracle_id: "cb91e842-9f06-4863-a328-2cabe1bcfe27",
    scryfall_id: "6e982bf8-382f-4987-bc39-28e1ce290340",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Ballroom",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
