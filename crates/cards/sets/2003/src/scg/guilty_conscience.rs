//! Guilty Conscience — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage, this Aura deals that much damage to that creature.
//! Set: SCG #17 — Scourge | Scryfall ID: 67b8701c-0f03-4ad0-9097-3caf885abd59 | Oracle ID: a1439ace-5880-4fb8-a321-ec8063e7ed0e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUILTY_CONSCIENCE,
    oracle_id = "a1439ace-5880-4fb8-a321-ec8063e7ed0e",
    scryfall_id = "67b8701c-0f03-4ad0-9097-3caf885abd59",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Guilty Conscience",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
