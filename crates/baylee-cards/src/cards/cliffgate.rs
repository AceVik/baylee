//! Cliffgate — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color other than red.
//! Oracle: {T}: Add {R} or one mana of the chosen color.
//! Set: CLB #350 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 557470dc-c594-4b26-81b9-356bedb0c215 | Oracle ID: 1999b5ac-21fb-4d99-ad72-58bf507f9a59
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 357,
    oracle_id: "1999b5ac-21fb-4d99-ad72-58bf507f9a59",
    scryfall_id: "557470dc-c594-4b26-81b9-356bedb0c215",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Cliffgate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
