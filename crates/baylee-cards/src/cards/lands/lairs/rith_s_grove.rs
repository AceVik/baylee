//! Rith's Grove — (no cost) — Land — Lair
//! Oracle: When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.
//! Oracle: {T}: Add {R}, {G}, or {W}.
//! Set: DMR #255 — Dominaria Remastered | Scryfall ID: 8a605ce4-ede6-44dd-a2ea-e953902be6bd | Oracle ID: e13289e5-370b-435b-a38e-cf57c3078cec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 924,
    oracle_id: "e13289e5-370b-435b-a38e-cf57c3078cec",
    scryfall_id: "8a605ce4-ede6-44dd-a2ea-e953902be6bd",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Rith's Grove",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LAIR],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
