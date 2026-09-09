//! Gingerbread Cabin — (no cost) — Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Oracle: This land enters tapped unless you control three or more other Forests.
//! Oracle: When this land enters untapped, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
//! Set: C21 #290 — Commander 2021 | Scryfall ID: 3b583cc8-95e6-4772-afe3-d405b65836e0 | Oracle ID: fa98c367-0312-49c6-abef-72e5ead4cc7d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 549,
    oracle_id: "fa98c367-0312-49c6-abef-72e5ead4cc7d",
    scryfall_id: "3b583cc8-95e6-4772-afe3-d405b65836e0",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Gingerbread Cabin",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
