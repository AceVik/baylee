//! Cinder Glade — (no cost) — Land — Mountain Forest
//! Oracle: ({T}: Add {R} or {G}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #230 — Marvel Super Heroes Commander | Scryfall ID: ec93087a-5728-40e8-8625-a1d175d5252c | Oracle ID: dfac0258-e148-4d7d-8ded-fc2466d9caa6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 351,
    oracle_id: "dfac0258-e148-4d7d-8ded-fc2466d9caa6",
    scryfall_id: "ec93087a-5728-40e8-8625-a1d175d5252c",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Cinder Glade",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
