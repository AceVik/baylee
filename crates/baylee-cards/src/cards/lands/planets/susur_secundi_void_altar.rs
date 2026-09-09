//! Susur Secundi, Void Altar — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {1}{B}, {T}, Pay 2 life, Sacrifice a creature: Draw cards equal to the sacrificed creature's power. Activate only as a sorcery.
//! Set: EOE #259 — Edge of Eternities | Scryfall ID: aefb8c0d-2bc6-4bec-851e-0137b4abfb22 | Oracle ID: 50d6cadc-07e4-479e-90f4-e3a20f769bab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1125,
    oracle_id: "50d6cadc-07e4-479e-90f4-e3a20f769bab",
    scryfall_id: "aefb8c0d-2bc6-4bec-851e-0137b4abfb22",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Susur Secundi, Void Altar",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLANET],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
