//! Creosote Heath — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {G} or {W}.
//! Set: OTJ #255 — Outlaws of Thunder Junction | Scryfall ID: c5523dac-7aa0-4486-89c8-3b22a1411f26 | Oracle ID: c116b787-5f7e-47ef-a694-58709770dd32
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 384,
    oracle_id: "c116b787-5f7e-47ef-a694-58709770dd32",
    scryfall_id: "c5523dac-7aa0-4486-89c8-3b22a1411f26",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[
    face! {
        name: "Creosote Heath",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
