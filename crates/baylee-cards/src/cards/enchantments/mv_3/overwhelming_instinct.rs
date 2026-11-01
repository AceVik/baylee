//! Overwhelming Instinct — {2}{G} — Enchantment
//! Oracle: Whenever you attack with three or more creatures, draw a card.
//! Set: ONS #276 — Onslaught | Scryfall ID: 2d9e3793-7ddc-45c5-b25d-acd5cb96026f | Oracle ID: 9ba89c25-5302-414f-a2ec-436b8981999d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERWHELMING_INSTINCT,
    oracle_id = "9ba89c25-5302-414f-a2ec-436b8981999d",
    scryfall_id = "2d9e3793-7ddc-45c5-b25d-acd5cb96026f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Overwhelming Instinct",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
