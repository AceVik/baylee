//! Altar of Bone — {G}{W} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a creature card, reveal it, put it into your hand, then shuffle.
//! Set: ICE #281 — Ice Age | Scryfall ID: 75d5b014-8675-4d91-a539-ac5c31d44b35 | Oracle ID: 607a3f5f-4a36-4077-b5cf-40ffd52fa580

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALTAR_OF_BONE,
    oracle_id = "607a3f5f-4a36-4077-b5cf-40ffd52fa580",
    scryfall_id = "75d5b014-8675-4d91-a539-ac5c31d44b35",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Altar of Bone",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::CREATURE,
        finds: &[Find::HAND],
        optional: false,
    }])],
);
