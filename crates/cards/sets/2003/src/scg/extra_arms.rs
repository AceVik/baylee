//! Extra Arms — {4}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature attacks, it deals 2 damage to any target.
//! Set: SCG #92 — Scourge | Scryfall ID: 28efa11c-6aeb-4c22-bbb3-b41f26d65c65 | Oracle ID: 1470f614-88eb-4d3c-bc4f-270dfb512b6c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXTRA_ARMS,
    oracle_id = "1470f614-88eb-4d3c-bc4f-270dfb512b6c",
    scryfall_id = "28efa11c-6aeb-4c22-bbb3-b41f26d65c65",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Extra Arms",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
