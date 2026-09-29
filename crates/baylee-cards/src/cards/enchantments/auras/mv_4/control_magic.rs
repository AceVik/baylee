//! Control Magic — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: You control enchanted creature.
//! Set: CMA #34 — Commander Anthology | Scryfall ID: 84992800-9bad-4598-afd4-f1e59d2e0956 | Oracle ID: cd0d7141-46d2-4aa3-bc77-6b3b4513803e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONTROL_MAGIC,
    oracle_id = "cd0d7141-46d2-4aa3-bc77-6b3b4513803e",
    scryfall_id = "84992800-9bad-4598-afd4-f1e59d2e0956",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Control Magic",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
