//! Rhystic Circle — {2}{W}{W} — Enchantment
//! Oracle: {1}: Any player may pay {1}. If no one does, the next time a source of your choice would deal damage to you this turn, prevent that damage.
//! Set: PCY #19 — Prophecy | Scryfall ID: 4a76711e-b508-4bb7-a87c-911a11905af3 | Oracle ID: bffb7d30-b9a4-4529-a3fc-c82d6cd4b599
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_CIRCLE,
    oracle_id = "bffb7d30-b9a4-4529-a3fc-c82d6cd4b599",
    scryfall_id = "4a76711e-b508-4bb7-a87c-911a11905af3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Rhystic Circle",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
