//! Energy Field — {1}{U} — Enchantment
//! Oracle: Prevent all damage that would be dealt to you by sources you don't control.
//! Oracle: When a card is put into your graveyard from anywhere, sacrifice this enchantment.
//! Set: USG #73 — Urza's Saga | Scryfall ID: 81ff5770-b207-41e1-97b7-b9347c72b407 | Oracle ID: fbf7a7f4-9915-4a53-84b3-e55e9f487323
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_FIELD,
    oracle_id = "fbf7a7f4-9915-4a53-84b3-e55e9f487323",
    scryfall_id = "81ff5770-b207-41e1-97b7-b9347c72b407",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Energy Field",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
