//! Azor's Gateway // Sanctum of the Sun — {2} — Legendary Artifact // Legendary Land
//! Set: RIX #176 — Rivals of Ixalan | Scryfall ID: 303d51ab-b9c4-4647-950f-291daabe7b81 | Oracle ID: c0cbb347-b060-43ce-a9c5-8c835be3cf1b
//! Face: Azor's Gateway — {2} — Legendary Artifact
//! Face: Sanctum of the Sun —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 247,
    oracle_id: "c0cbb347-b060-43ce-a9c5-8c835be3cf1b",
    scryfall_id: "303d51ab-b9c4-4647-950f-291daabe7b81",
    faces: &[
    face! {
        name: "Azor's Gateway",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Sanctum of the Sun",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
