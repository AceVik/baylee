//! Thousand Moons Smithy // Barracks of the Thousand — {2}{W}{W} — Legendary Artifact // Legendary Artifact Land
//! Set: LCI #39 — The Lost Caverns of Ixalan | Scryfall ID: 4a6bec46-1acd-4726-b8d9-3045ac6a2ea2 | Oracle ID: 32af5f7b-a970-484a-9aff-226749551d32
//! Face: Thousand Moons Smithy — {2}{W}{W} — Legendary Artifact
//! Face: Barracks of the Thousand —  — Legendary Artifact Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1188,
    oracle_id: "32af5f7b-a970-484a-9aff-226749551d32",
    scryfall_id: "4a6bec46-1acd-4726-b8d9-3045ac6a2ea2",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Thousand Moons Smithy",
        mana_cost: baylee_core::mana!("{2}{W}{W}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Barracks of the Thousand",
        types: TypeSet::ARTIFACT.union(TypeSet::LAND),
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
