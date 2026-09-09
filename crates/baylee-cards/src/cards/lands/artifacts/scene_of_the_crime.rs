//! Scene of the Crime — (no cost) — Artifact Land — Clue
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Tap an untapped creature you control: Add one mana of any color.
//! Oracle: {2}, Sacrifice this land: Draw a card.
//! Set: MKM #267 — Murders at Karlov Manor | Scryfall ID: de039992-631b-4feb-a522-acdb0a6d1f26 | Oracle ID: ba11a517-1dbd-4797-9f5e-46ce0f6c77c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 972,
    oracle_id: "ba11a517-1dbd-4797-9f5e-46ce0f6c77c0",
    scryfall_id: "de039992-631b-4feb-a522-acdb0a6d1f26",
    faces: &[
    face! {
        name: "Scene of the Crime",
        types: TypeSet::ARTIFACT.union(TypeSet::LAND),
        subtypes: &[subtypes::artifact::CLUE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
