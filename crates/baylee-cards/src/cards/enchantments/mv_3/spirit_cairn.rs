//! Spirit Cairn — {2}{W} — Enchantment
//! Oracle: Whenever a player discards a card, you may pay {W}. If you do, create a 1/1 white Spirit creature token with flying.
//! Set: C20 #100 — Commander 2020 | Scryfall ID: 9b0f860c-029c-4856-a999-05b3306c7d46 | Oracle ID: ae896d87-59c7-4d8d-a137-9e471d3e174e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRIT_CAIRN,
    oracle_id = "ae896d87-59c7-4d8d-a137-9e471d3e174e",
    scryfall_id = "9b0f860c-029c-4856-a999-05b3306c7d46",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spirit Cairn",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
