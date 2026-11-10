//! Mana Maze — {1}{U} — Enchantment
//! Oracle: Players can't cast spells that share a color with the spell most recently cast this turn.
//! Set: INV #59 — Invasion | Scryfall ID: 0d62cc17-8fa3-495c-a098-ffbbec89fa53 | Oracle ID: 5b6f9d55-eae8-48fd-a1d1-d56939adaf3a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_MAZE,
    oracle_id = "5b6f9d55-eae8-48fd-a1d1-d56939adaf3a",
    scryfall_id = "0d62cc17-8fa3-495c-a098-ffbbec89fa53",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mana Maze",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
