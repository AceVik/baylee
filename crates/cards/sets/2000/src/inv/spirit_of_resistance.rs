//! Spirit of Resistance — {2}{W} — Enchantment
//! Oracle: As long as you control a permanent of each color, prevent all damage that would be dealt to you.
//! Set: INV #38 — Invasion | Scryfall ID: 5fb66439-df73-4a01-a8d4-6f2334297fdf | Oracle ID: b075b645-2e5a-4d91-9440-02db4e6f5d08
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRIT_OF_RESISTANCE,
    oracle_id = "b075b645-2e5a-4d91-9440-02db4e6f5d08",
    scryfall_id = "5fb66439-df73-4a01-a8d4-6f2334297fdf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spirit of Resistance",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
