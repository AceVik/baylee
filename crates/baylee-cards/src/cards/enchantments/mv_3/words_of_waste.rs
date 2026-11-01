//! Words of Waste — {2}{B} — Enchantment
//! Oracle: {1}: The next time you would draw a card this turn, each opponent discards a card instead.
//! Set: ONS #182 — Onslaught | Scryfall ID: d2dcb8ed-23e7-4cee-9f43-042232c6035a | Oracle ID: 3fe5cfcd-b25f-49d8-9f60-8b4c388a9c68
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WASTE,
    oracle_id = "3fe5cfcd-b25f-49d8-9f60-8b4c388a9c68",
    scryfall_id = "d2dcb8ed-23e7-4cee-9f43-042232c6035a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Words of Waste",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
