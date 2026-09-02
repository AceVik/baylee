//! Conqueror's Galleon // Conqueror's Foothold — {4} — Artifact — Vehicle // Land
//! Set: XLN #234 — Ixalan | Scryfall ID: 02bb4b2a-3c9f-48ab-b2a5-ae31f06b82d9 | Oracle ID: 88b18901-50cd-461c-b1bc-be900210be8e
//! Face: Conqueror's Galleon — {4} — Artifact — Vehicle
//! Face: Conqueror's Foothold —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 369,
    oracle_id: "88b18901-50cd-461c-b1bc-be900210be8e",
    scryfall_id: "02bb4b2a-3c9f-48ab-b2a5-ae31f06b82d9",
    faces: &[
    face! {
        name: "Conqueror's Galleon",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        subtypes: &[subtypes::artifact::VEHICLE],
        power: Some(2),
        toughness: Some(10),
    },
    face! {
        name: "Conqueror's Foothold",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
