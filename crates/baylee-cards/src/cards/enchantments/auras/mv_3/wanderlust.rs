//! Wanderlust — {2}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, this Aura deals 1 damage to that player.
//! Set: ME1 #137 — Masters Edition | Scryfall ID: 089b7137-8824-449b-890f-97e761950a17 | Oracle ID: 73bddfdb-d1fb-4038-b676-201f6b82beb0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WANDERLUST,
    oracle_id = "73bddfdb-d1fb-4038-b676-201f6b82beb0",
    scryfall_id = "089b7137-8824-449b-890f-97e761950a17",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wanderlust",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
