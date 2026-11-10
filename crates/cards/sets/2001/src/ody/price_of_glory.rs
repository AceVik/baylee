//! Price of Glory — {2}{R} — Enchantment
//! Oracle: Whenever a player taps a land for mana, if it's not that player's turn, destroy that land.
//! Set: ODY #214 — Odyssey | Scryfall ID: 785ccfe1-04f1-4d2e-9e30-2ae3efe7213a | Oracle ID: 35064a72-c052-496f-8f1a-6dbc5e617d3f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRICE_OF_GLORY,
    oracle_id = "35064a72-c052-496f-8f1a-6dbc5e617d3f",
    scryfall_id = "785ccfe1-04f1-4d2e-9e30-2ae3efe7213a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Price of Glory",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
