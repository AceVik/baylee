//! Forgotten Harvest — {1}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may exile a land card from your graveyard. If you do, put a +1/+1 counter on target creature.
//! Set: PCY #114 — Prophecy | Scryfall ID: 9fefbace-03cb-43db-a221-0be2b8784357 | Oracle ID: 9e5de2ed-e8a1-4d52-832f-c4a1bfdf2ae5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGOTTEN_HARVEST,
    oracle_id = "9e5de2ed-e8a1-4d52-832f-c4a1bfdf2ae5",
    scryfall_id = "9fefbace-03cb-43db-a221-0be2b8784357",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Forgotten Harvest",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
