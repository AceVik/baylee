//! The Everflowing Well // The Myriad Pools — {2}{U} — Legendary Artifact // Legendary Artifact Land
//! Set: LCI #56 — The Lost Caverns of Ixalan | Scryfall ID: bf573fb7-fa6c-4df7-8e5e-1e071585361e | Oracle ID: 1f57a9f1-6b95-4395-bdf0-c5289b786ab1
//! Face: The Everflowing Well — {2}{U} — Legendary Artifact
//! Face: The Myriad Pools —  — Legendary Artifact Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1170,
    oracle_id: "1f57a9f1-6b95-4395-bdf0-c5289b786ab1",
    scryfall_id: "bf573fb7-fa6c-4df7-8e5e-1e071585361e",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "The Everflowing Well",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "The Myriad Pools",
        types: TypeSet::ARTIFACT.union(TypeSet::LAND),
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
