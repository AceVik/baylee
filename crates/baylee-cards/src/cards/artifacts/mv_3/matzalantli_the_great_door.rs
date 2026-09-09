//! Matzalantli, the Great Door // The Core — {3} — Legendary Artifact // Legendary Land
//! Set: LCI #256 — The Lost Caverns of Ixalan | Scryfall ID: b4c31b29-06ba-436d-a3d9-18f4796c39be | Oracle ID: 16182e01-22ff-4786-985d-919b47c4aa4d
//! Face: Matzalantli, the Great Door — {3} — Legendary Artifact
//! Face: The Core —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 741,
    oracle_id: "16182e01-22ff-4786-985d-919b47c4aa4d",
    scryfall_id: "b4c31b29-06ba-436d-a3d9-18f4796c39be",
    faces: &[
    face! {
        name: "Matzalantli, the Great Door",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "The Core",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
