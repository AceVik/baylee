//! Battle of Wits — {3}{U}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have 200 or more cards in your library, you win the game.
//! Set: M13 #44 — Magic 2013 | Scryfall ID: b4be15a4-693f-4e22-a46c-38bb440c073c | Oracle ID: 9b544415-841d-4038-841b-efb0cede5c24
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BATTLE_OF_WITS,
    oracle_id = "9b544415-841d-4038-841b-efb0cede5c24",
    scryfall_id = "b4be15a4-693f-4e22-a46c-38bb440c073c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Battle of Wits",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
