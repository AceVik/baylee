//! Crosis's Catacombs — (no cost) — Land — Lair
//! Oracle: When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.
//! Oracle: {T}: Add {U}, {B}, or {R}.
//! Set: DMR #242 — Dominaria Remastered | Scryfall ID: 52ce462b-e4de-4972-9e96-2e3d3a20d1fc | Oracle ID: e9a7dede-3968-4b0e-a707-419d46a6fec9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 385,
    oracle_id: "e9a7dede-3968-4b0e-a707-419d46a6fec9",
    scryfall_id: "52ce462b-e4de-4972-9e96-2e3d3a20d1fc",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Crosis's Catacombs",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LAIR],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
