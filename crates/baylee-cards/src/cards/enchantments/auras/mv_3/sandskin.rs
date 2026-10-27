//! Sandskin — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Prevent all combat damage that would be dealt to and dealt by enchanted creature.
//! Set: ONS #52 — Onslaught | Scryfall ID: 80b59844-c9d4-4bc1-86e6-4cc596d9165d | Oracle ID: 13628636-7fc1-426e-abac-07fc35caf81b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SANDSKIN,
    oracle_id = "13628636-7fc1-426e-abac-07fc35caf81b",
    scryfall_id = "80b59844-c9d4-4bc1-86e6-4cc596d9165d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sandskin",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
