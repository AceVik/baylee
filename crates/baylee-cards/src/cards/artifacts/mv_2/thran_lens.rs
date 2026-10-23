//! Thran Lens — {2} — Artifact
//! Oracle: All permanents are colorless.
//! Set: ULG #133 — Urza's Legacy | Scryfall ID: 200c3666-5ba0-4f0a-adbe-a97af0aa28d1 | Oracle ID: 0ac723bf-ce86-4028-a693-0ef498e4c82d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRAN_LENS,
    oracle_id = "0ac723bf-ce86-4028-a693-0ef498e4c82d",
    scryfall_id = "200c3666-5ba0-4f0a-adbe-a97af0aa28d1",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thran Lens",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::SetColor(ColorSet::EMPTY),
    )],
);
