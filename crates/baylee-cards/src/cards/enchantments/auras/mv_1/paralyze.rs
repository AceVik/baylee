//! Paralyze — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, tap enchanted creature.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may pay {4}. If the player does, untap the creature.
//! Set: VMA #132 — Vintage Masters | Scryfall ID: fe85610f-b17e-4628-8f6b-544ea7b34327 | Oracle ID: e9b4e857-39e5-4a06-89ad-3dd94b3f252a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARALYZE,
    oracle_id = "e9b4e857-39e5-4a06-89ad-3dd94b3f252a",
    scryfall_id = "fe85610f-b17e-4628-8f6b-544ea7b34327",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Paralyze",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
