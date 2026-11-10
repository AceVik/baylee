//! Great Wall — {2}{W} — Enchantment
//! Oracle: Creatures with plainswalk can be blocked as though they didn't have plainswalk.
//! Set: LEG #17 — Legends | Scryfall ID: cd860a1d-aa17-4579-b9b1-d101d2416387 | Oracle ID: 232d7613-4156-4bf5-9746-97fc79e4193d
// IMPLEMENTED — a layer-6 static strips plainswalk from every creature that
// has it, so it can be blocked as though it didn't have plainswalk.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREAT_WALL,
    oracle_id = "232d7613-4156-4bf5-9746-97fc79e4193d",
    scryfall_id = "cd860a1d-aa17-4579-b9b1-d101d2416387",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Great Wall",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::PLAINSWALK)]),
        Modifier::RemoveKeyword(KeywordSet::PLAINSWALK)
    )],
);
