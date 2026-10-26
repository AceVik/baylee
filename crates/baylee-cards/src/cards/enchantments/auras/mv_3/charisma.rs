//! Charisma — {U}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage to a creature, gain control of the other creature for as long as this Aura remains on the battlefield.
//! Set: MMQ #66 — Mercadian Masques | Scryfall ID: 63565b03-28e9-4534-b085-d5803e2623bb | Oracle ID: d2abfcd1-c00c-43fa-a9ca-a9577b3c8a60
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHARISMA,
    oracle_id = "d2abfcd1-c00c-43fa-a9ca-a9577b3c8a60",
    scryfall_id = "63565b03-28e9-4534-b085-d5803e2623bb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Charisma",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
