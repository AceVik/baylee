//! Decaying Soil — {1}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, exile a card from your graveyard.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this enchantment has "Whenever a nontoken creature is put into your graveyard from the battlefield, you may pay {1}. If you do, return that card to your hand."
//! Set: ODY #127 — Odyssey | Scryfall ID: f2bea0ec-52e0-4f45-a445-8805dc6ed596 | Oracle ID: 3a58d68c-7683-4543-a929-8ef12641a564
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECAYING_SOIL,
    oracle_id = "3a58d68c-7683-4543-a929-8ef12641a564",
    scryfall_id = "f2bea0ec-52e0-4f45-a445-8805dc6ed596",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Decaying Soil",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
