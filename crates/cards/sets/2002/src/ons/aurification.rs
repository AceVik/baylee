//! Aurification — {2}{W}{W} — Enchantment
//! Oracle: Whenever a creature deals damage to you, put a gold counter on it.
//! Oracle: Each creature with a gold counter on it is a Wall in addition to its other creature types and has defender. (Those creatures can't attack.)
//! Oracle: When this enchantment leaves the battlefield, remove all gold counters from all creatures.
//! Set: ONS #6 — Onslaught | Scryfall ID: 93d9e9ea-9f88-4206-8960-b5ebe839ee16 | Oracle ID: 3d98a656-1737-486f-9b25-449b7c871d1f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AURIFICATION,
    oracle_id = "3d98a656-1737-486f-9b25-449b7c871d1f",
    scryfall_id = "93d9e9ea-9f88-4206-8960-b5ebe839ee16",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aurification",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
