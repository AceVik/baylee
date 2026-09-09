//! Sunken Citadel — (no cost) — Land — Cave
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {T}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.
//! Set: LCI #285 — The Lost Caverns of Ixalan | Scryfall ID: 3e1c9b1a-e306-47bb-9f68-2083660319c0 | Oracle ID: 508189e1-9cef-4f9c-8ff1-078c99a0f603
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1114,
    oracle_id: "508189e1-9cef-4f9c-8ff1-078c99a0f603",
    scryfall_id: "3e1c9b1a-e306-47bb-9f68-2083660319c0",
    faces: &[
    face! {
        name: "Sunken Citadel",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
