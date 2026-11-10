//! Helm of Awakening — {2} — Artifact
//! Oracle: Spells cost {1} less to cast.
//! Set: DMR #224 — Dominaria Remastered | Scryfall ID: e9e4ff9c-6244-43e1-b569-32a270c4e014 | Oracle ID: f2fcef65-4873-4b1f-a257-b8648097c990
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELM_OF_AWAKENING,
    oracle_id = "f2fcef65-4873-4b1f-a257-b8648097c990",
    scryfall_id = "e9e4ff9c-6244-43e1-b569-32a270c4e014",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Helm of Awakening",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(Filter::Any, Modifier::SpellsCostLess(1))],
);
