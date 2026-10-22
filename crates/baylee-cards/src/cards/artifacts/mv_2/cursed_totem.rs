//! Cursed Totem — {2} — Artifact
//! Oracle: Activated abilities of creatures can't be activated.
//! Set: MH2 #295 — Modern Horizons 2 | Scryfall ID: 5dde91a9-7d2d-4a7b-861a-3d1c16ec79d9 | Oracle ID: 6225a704-430a-4f56-ad87-0e8d87f285f5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CURSED_TOTEM,
    oracle_id = "6225a704-430a-4f56-ad87-0e8d87f285f5",
    scryfall_id = "5dde91a9-7d2d-4a7b-861a-3d1c16ec79d9",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cursed Totem",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(Filter::CREATURE, Modifier::CantActivate)],
);
