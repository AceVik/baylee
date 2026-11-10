//! Crevasse — {2}{R} — Enchantment
//! Oracle: Creatures with mountainwalk can be blocked as though they didn't have mountainwalk.
//! Set: LEG #138 — Legends | Scryfall ID: a432d6ae-a17f-484b-ad55-4b4b6674ba8d | Oracle ID: 6d03297b-fa54-438e-99c8-37f92163aeff
// IMPLEMENTED — a layer-6 static strips mountainwalk from every creature that
// has it, so it can be blocked as though it didn't have mountainwalk.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CREVASSE,
    oracle_id = "6d03297b-fa54-438e-99c8-37f92163aeff",
    scryfall_id = "a432d6ae-a17f-484b-ad55-4b4b6674ba8d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crevasse",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::CREATURE,
            Filter::HasKeyword(KeywordSet::MOUNTAINWALK)
        ]),
        Modifier::RemoveKeyword(KeywordSet::MOUNTAINWALK)
    )],
);
