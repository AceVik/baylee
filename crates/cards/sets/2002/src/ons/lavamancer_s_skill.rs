//! Lavamancer's Skill — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: This creature deals 1 damage to target creature."
//! Oracle: As long as enchanted creature is a Wizard, it has "{T}: This creature deals 2 damage to target creature."
//! Set: ONS #215 — Onslaught | Scryfall ID: 0d4dd156-a2c1-4fab-b9f4-3302a4e8835a | Oracle ID: e7819a6d-a518-4400-8e30-2a528878fd8a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAVAMANCER_S_SKILL,
    oracle_id = "e7819a6d-a518-4400-8e30-2a528878fd8a",
    scryfall_id = "0d4dd156-a2c1-4fab-b9f4-3302a4e8835a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lavamancer's Skill",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
