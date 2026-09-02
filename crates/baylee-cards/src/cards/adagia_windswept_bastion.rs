//! Adagia, Windswept Bastion — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {3}{W}, {T}: Create a token that's a copy of target artifact or enchantment you control, except it's legendary. Activate only as a sorcery.
//! Set: EOE #250 — Edge of Eternities | Scryfall ID: c634273a-94b0-4104-9d10-ae522ece1fc7 | Oracle ID: 70d35dbd-1d91-4a2a-a643-6870d168f4f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 206,
    oracle_id: "70d35dbd-1d91-4a2a-a643-6870d168f4f5",
    scryfall_id: "c634273a-94b0-4104-9d10-ae522ece1fc7",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Adagia, Windswept Bastion",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLANET],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
