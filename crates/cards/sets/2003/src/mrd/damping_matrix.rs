//! Damping Matrix — {3} — Artifact
//! Oracle: Activated abilities of artifacts and creatures can't be activated unless they're mana abilities.
//! Set: MM3 #218 — Modern Masters 2017 | Scryfall ID: d7c89492-ef45-460e-9c78-83c8c8c80fe2 | Oracle ID: 10403a8c-2534-421e-8809-5fc019623452
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAMPING_MATRIX,
    oracle_id = "10403a8c-2534-421e-8809-5fc019623452",
    scryfall_id = "d7c89492-ef45-460e-9c78-83c8c8c80fe2",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Damping Matrix",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::ARTIFACT_OR_CREATURE,
        Modifier::CantActivateNonMana,
    )],
);
