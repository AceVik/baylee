//! Living Wish — {1}{G} — Sorcery
//! Oracle: You may reveal a creature or land card you own from outside the game and put it into your hand. Exile Living Wish.
//! Set: A25 #179 — Masters 25 | Scryfall ID: d505d319-093d-47af-8ee9-fafae3885aa0 | Oracle ID: b9d186f5-1c92-4539-a5b5-e7fbd4b726fe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIVING_WISH,
    oracle_id = "b9d186f5-1c92-4539-a5b5-e7fbd4b726fe",
    scryfall_id = "d505d319-093d-47af-8ee9-fafae3885aa0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Living Wish",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
