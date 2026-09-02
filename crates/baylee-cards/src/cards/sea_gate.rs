//! Sea Gate — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color other than blue.
//! Oracle: {T}: Add {U} or one mana of the chosen color.
//! Set: CLB #359 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: d97f31e1-bcaf-4316-a2de-49e2cf7566ec | Oracle ID: b574c540-9f8a-4fd4-8809-d02c9b099ddc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 978,
    oracle_id: "b574c540-9f8a-4fd4-8809-d02c9b099ddc",
    scryfall_id: "d97f31e1-bcaf-4316-a2de-49e2cf7566ec",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Sea Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
