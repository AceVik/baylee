//! Words of Worship — {2}{W} — Enchantment
//! Oracle: {1}: The next time you would draw a card this turn, you gain 5 life instead.
//! Set: ONS #61 — Onslaught | Scryfall ID: 0ea5c6e0-8361-4214-997b-32a66b19fae9 | Oracle ID: f1fad5ab-d087-4912-b1b9-42733fa34707
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WORSHIP,
    oracle_id = "f1fad5ab-d087-4912-b1b9-42733fa34707",
    scryfall_id = "0ea5c6e0-8361-4214-997b-32a66b19fae9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Words of Worship",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
