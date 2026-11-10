//! Infernal Genesis — {4}{B}{B} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player mills a card. Then they create X 1/1 black Minion creature tokens, where X is the milled card's mana value.
//! Set: PCY #68 — Prophecy | Scryfall ID: 1a63d16e-319d-46f4-a28c-895b36605ee6 | Oracle ID: c3ce3473-e3bc-4b7c-ae57-957e6c40dc7a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFERNAL_GENESIS,
    oracle_id = "c3ce3473-e3bc-4b7c-ae57-957e6c40dc7a",
    scryfall_id = "1a63d16e-319d-46f4-a28c-895b36605ee6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Infernal Genesis",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
