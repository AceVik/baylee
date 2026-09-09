//! The Lonely Mountain — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped unless you control an Equipment.
//! Oracle: {4}{R}, {T}: Create a 2/2 red Dwarf creature token. This ability costs {1} less to activate for each Equipment you control. Activate only as a sorcery.
//! Set: HOB #187 — The Hobbit | Scryfall ID: b39ebc4d-a01a-4401-ab3a-bf6142c93b47 | Oracle ID: 3678c06f-8a33-4a6d-bf20-5b92d5c05a95
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1176,
    oracle_id: "3678c06f-8a33-4a6d-bf20-5b92d5c05a95",
    scryfall_id: "b39ebc4d-a01a-4401-ab3a-bf6142c93b47",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "The Lonely Mountain",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
