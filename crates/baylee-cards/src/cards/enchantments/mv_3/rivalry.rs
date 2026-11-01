//! Rivalry — {2}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, if that player controls more lands than each other player, this enchantment deals 2 damage to them.
//! Set: ULG #90 — Urza's Legacy | Scryfall ID: 3b79677b-d5b9-47c7-a5f5-45446d5cddff | Oracle ID: da77cf1b-eaeb-4a60-aa16-a1217ce8b550
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVALRY,
    oracle_id = "da77cf1b-eaeb-4a60-aa16-a1217ce8b550",
    scryfall_id = "3b79677b-d5b9-47c7-a5f5-45446d5cddff",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rivalry",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
