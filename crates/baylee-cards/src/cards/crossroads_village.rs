//! Crossroads Village — (no cost) — Land — Town
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Set: FIN #276 — Final Fantasy | Scryfall ID: 64db46d4-f91f-49cc-971c-b8e19f0c4ea9 | Oracle ID: b26cfeb0-7bbe-4d93-8eed-e832f175a80c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 386,
    oracle_id: "b26cfeb0-7bbe-4d93-8eed-e832f175a80c",
    scryfall_id: "64db46d4-f91f-49cc-971c-b8e19f0c4ea9",
    faces: &[
    face! {
        name: "Crossroads Village",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
