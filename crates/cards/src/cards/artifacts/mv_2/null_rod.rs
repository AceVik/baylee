//! Null Rod — {2} — Artifact
//! Oracle: Activated abilities of artifacts can't be activated.
//! Set: VMA #278 — Vintage Masters | Scryfall ID: 73ac9f52-e6ff-4e6f-9733-fe24a5fb4b4e | Oracle ID: 2f83ca86-e23d-40f7-8085-6928d8cfef9b
// IMPLEMENTED — Prevents activated abilities of artifacts (including mana abilities) from being activated.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NULL_ROD,
    oracle_id = "2f83ca86-e23d-40f7-8085-6928d8cfef9b",
    scryfall_id = "73ac9f52-e6ff-4e6f-9733-fe24a5fb4b4e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Null Rod",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(Filter::ARTIFACT, Modifier::CantActivate),],
);
