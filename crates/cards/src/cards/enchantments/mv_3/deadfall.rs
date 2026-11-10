//! Deadfall — {2}{G} — Enchantment
//! Oracle: Creatures with forestwalk can be blocked as though they didn't have forestwalk.
//! Set: LEG #181 — Legends | Scryfall ID: 0d78f0fc-3ab2-46ee-b5a9-55ae97d08c1a | Oracle ID: 6a7d6bb7-d6ad-48a4-8923-4083e0c90786
// IMPLEMENTED — a layer-6 static strips forestwalk from every creature that
// has it, so it can be blocked as though it didn't have forestwalk.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEADFALL,
    oracle_id = "6a7d6bb7-d6ad-48a4-8923-4083e0c90786",
    scryfall_id = "0d78f0fc-3ab2-46ee-b5a9-55ae97d08c1a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deadfall",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FORESTWALK)]),
        Modifier::RemoveKeyword(KeywordSet::FORESTWALK)
    )],
);
