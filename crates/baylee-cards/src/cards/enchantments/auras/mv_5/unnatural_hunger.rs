//! Unnatural Hunger — {3}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, this Aura deals damage equal to that creature's power to that player unless they sacrifice another creature of their choice.
//! Set: MMQ #169 — Mercadian Masques | Scryfall ID: 3985f240-4289-4d48-978c-bb2ce2b54c36 | Oracle ID: d37a361e-83e8-4f0a-bd09-3135fb918471
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNNATURAL_HUNGER,
    oracle_id = "d37a361e-83e8-4f0a-bd09-3135fb918471",
    scryfall_id = "3985f240-4289-4d48-978c-bb2ce2b54c36",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Unnatural Hunger",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
