//! Evendo, Waking Haven — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {G}, {T}: Add {G} for each creature you control.
//! Set: EOE #253 — Edge of Eternities | Scryfall ID: 2fa09104-acbe-4410-b101-2fe6ac28efde | Oracle ID: 83161d59-2520-4741-9328-e2a4a8b5d5bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 478,
    oracle_id: "83161d59-2520-4741-9328-e2a4a8b5d5bc",
    scryfall_id: "2fa09104-acbe-4410-b101-2fe6ac28efde",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Evendo, Waking Haven",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLANET],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
