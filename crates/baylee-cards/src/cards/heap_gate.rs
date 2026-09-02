//! Heap Gate — (no cost) — Land — Gate
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {1}, {T}, Tap an untapped Gate you control: Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: CLB #354 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 68489d65-1978-48b1-a903-2ef38c583239 | Oracle ID: 35922a30-6b84-44dd-a2f0-306554a1ae90
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 611,
    oracle_id: "35922a30-6b84-44dd-a2f0-306554a1ae90",
    scryfall_id: "68489d65-1978-48b1-a903-2ef38c583239",
    faces: &[
    face! {
        name: "Heap Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
