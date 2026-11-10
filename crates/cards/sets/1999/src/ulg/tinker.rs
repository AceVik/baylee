//! Tinker — {2}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice an artifact.
//! Oracle: Search your library for an artifact card, put that card onto the battlefield, then shuffle.
//! Set: ULG #45 — Urza's Legacy | Scryfall ID: 7da23b15-dfb8-4267-9b33-d7a4c035c434 | Oracle ID: 254878f0-be90-4653-a395-0c41258fceaf
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TINKER,
    oracle_id = "254878f0-be90-4653-a395-0c41258fceaf",
    scryfall_id = "7da23b15-dfb8-4267-9b33-d7a4c035c434",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tinker",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::ARTIFACT)],
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::ARTIFACT,
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }])],
);
