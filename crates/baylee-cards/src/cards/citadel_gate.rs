//! Citadel Gate — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color other than white.
//! Oracle: {T}: Add {W} or one mana of the chosen color.
//! Set: CLB #349 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: c0e6f002-9a10-49a1-8604-2b8ff57732dd | Oracle ID: 15f1fe23-5af4-4fc4-8cde-2e0bf9f9be0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 353,
    oracle_id: "15f1fe23-5af4-4fc4-8cde-2e0bf9f9be0c",
    scryfall_id: "c0e6f002-9a10-49a1-8604-2b8ff57732dd",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Citadel Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
