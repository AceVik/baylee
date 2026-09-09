//! Uthros, Titanic Godcore — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {U}, {T}: Add {U} for each artifact you control.
//! Set: EOE #260 — Edge of Eternities | Scryfall ID: 11da39d6-cfa6-498d-91b1-11454cc7e5a3 | Oracle ID: df08ac72-010f-42f8-beb3-6d645c638e1e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1273,
    oracle_id: "df08ac72-010f-42f8-beb3-6d645c638e1e",
    scryfall_id: "11da39d6-cfa6-498d-91b1-11454cc7e5a3",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Uthros, Titanic Godcore",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLANET],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
