//! Lush Oasis — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {G} or {U}.
//! Set: OTJ #261 — Outlaws of Thunder Junction | Scryfall ID: 988e44c5-4632-4ebb-b6ae-c3886e49d637 | Oracle ID: b6a965eb-cffb-41c1-925a-7cf3e8e2f248
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 728,
    oracle_id: "b6a965eb-cffb-41c1-925a-7cf3e8e2f248",
    scryfall_id: "988e44c5-4632-4ebb-b6ae-c3886e49d637",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Lush Oasis",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
