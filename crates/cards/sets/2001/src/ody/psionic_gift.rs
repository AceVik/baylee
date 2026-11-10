//! Psionic Gift — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: This creature deals 1 damage to any target."
//! Set: ODY #95 — Odyssey | Scryfall ID: 862fc18f-90d2-430d-aff2-47c1483dee9d | Oracle ID: 842bb10a-f056-4db8-a341-2d20796b99ba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PSIONIC_GIFT,
    oracle_id = "842bb10a-f056-4db8-a341-2d20796b99ba",
    scryfall_id = "862fc18f-90d2-430d-aff2-47c1483dee9d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psionic Gift",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
