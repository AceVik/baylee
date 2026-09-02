//! Treva's Ruins — (no cost) — Land — Lair
//! Oracle: When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.
//! Oracle: {T}: Add {G}, {W}, or {U}.
//! Set: DMR #260 — Dominaria Remastered | Scryfall ID: 4f39366c-00df-4528-a324-6a89d0c53f0a | Oracle ID: 7b2c7758-2b89-49ff-8838-8dc9880c7209
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1230,
    oracle_id: "7b2c7758-2b89-49ff-8838-8dc9880c7209",
    scryfall_id: "4f39366c-00df-4528-a324-6a89d0c53f0a",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Treva's Ruins",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LAIR],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
