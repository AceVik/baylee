//! Animate Wall — {W} — Enchantment — Aura
//! Oracle: Enchant Wall
//! Oracle: Enchanted Wall can attack as though it didn't have defender.
//! Set: ME1 #2 — Masters Edition | Scryfall ID: 0fa5725c-fcf8-4f84-9093-75b454b4755f | Oracle ID: c7a6a165-b709-46e0-ae42-6f69a17c0621
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANIMATE_WALL,
    oracle_id = "c7a6a165-b709-46e0-ae42-6f69a17c0621",
    scryfall_id = "0fa5725c-fcf8-4f84-9093-75b454b4755f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Animate Wall",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
