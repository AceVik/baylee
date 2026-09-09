//! Darigaaz's Caldera — (no cost) — Land — Lair
//! Oracle: When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.
//! Oracle: {T}: Add {B}, {R}, or {G}.
//! Set: DMR #243 — Dominaria Remastered | Scryfall ID: 27b1ad2d-f02e-4e6b-a49a-f65d174bb0fc | Oracle ID: 19b58ec9-bb88-4193-8ea8-c8f09ceec1ed
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 400,
    oracle_id: "19b58ec9-bb88-4193-8ea8-c8f09ceec1ed",
    scryfall_id: "27b1ad2d-f02e-4e6b-a49a-f65d174bb0fc",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Darigaaz's Caldera",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LAIR],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
