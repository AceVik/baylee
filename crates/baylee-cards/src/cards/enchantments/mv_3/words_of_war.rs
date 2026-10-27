//! Words of War — {2}{R} — Enchantment
//! Oracle: {1}: The next time you would draw a card this turn, this enchantment deals 2 damage to any target instead.
//! Set: ONS #244 — Onslaught | Scryfall ID: 2593a6a6-dc21-4742-acb8-f7092931b1ce | Oracle ID: 76d6d4cc-607f-4a33-a964-a38b5dbfe710
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WAR,
    oracle_id = "76d6d4cc-607f-4a33-a964-a38b5dbfe710",
    scryfall_id = "2593a6a6-dc21-4742-acb8-f7092931b1ce",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Words of War",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
