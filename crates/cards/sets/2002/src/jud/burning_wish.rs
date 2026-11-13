//! Burning Wish — {1}{R} — Sorcery
//! Oracle: You may reveal a sorcery card you own from outside the game and put it into your hand. Exile Burning Wish.
//! Set: CN2 #152 — Conspiracy: Take the Crown | Scryfall ID: 3abb0b65-b3de-4193-ae60-9b8ba55636b4 | Oracle ID: 13a884b5-8970-4d71-9082-a8b6c8db9a01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURNING_WISH,
    oracle_id = "13a884b5-8970-4d71-9082-a8b6c8db9a01",
    scryfall_id = "3abb0b65-b3de-4193-ae60-9b8ba55636b4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Burning Wish",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
