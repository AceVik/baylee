//! Thran Portal — (no cost) — Land — Gate
//! Oracle: This land enters tapped unless you control two or fewer other lands.
//! Oracle: As this land enters, choose a basic land type.
//! Oracle: This land is the chosen type in addition to its other types.
//! Oracle: Mana abilities of this land cost an additional 1 life to activate.
//! Set: DMU #259 — Dominaria United | Scryfall ID: ef074a2e-a387-4af8-a180-74b145d93992 | Oracle ID: 926ce6a2-7bdd-4380-ac65-bc902ba0c284
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1189,
    oracle_id: "926ce6a2-7bdd-4380-ac65-bc902ba0c284",
    scryfall_id: "ef074a2e-a387-4af8-a180-74b145d93992",
    faces: &[
    face! {
        name: "Thran Portal",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
