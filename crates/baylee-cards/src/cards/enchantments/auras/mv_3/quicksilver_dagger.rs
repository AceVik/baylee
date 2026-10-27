//! Quicksilver Dagger — {1}{U}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: This creature deals 1 damage to target player or planeswalker. You draw a card."
//! Set: DMR #195 — Dominaria Remastered | Scryfall ID: 138d2d9d-d5ab-400f-a714-cd38212a0dae | Oracle ID: 5c77bf76-bb0f-4e44-babe-0e7b7b555e1f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUICKSILVER_DAGGER,
    oracle_id = "5c77bf76-bb0f-4e44-babe-0e7b7b555e1f",
    scryfall_id = "138d2d9d-d5ab-400f-a714-cd38212a0dae",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Quicksilver Dagger",
        mana_cost = mana!("{1}{U}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
