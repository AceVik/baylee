//! Spiritual Focus — {1}{W} — Enchantment
//! Oracle: Whenever a spell or ability an opponent controls causes you to discard a card, you gain 2 life and you may draw a card.
//! Set: MMQ #49 — Mercadian Masques | Scryfall ID: 8521ae08-eb46-45ff-8fc4-62d8b07cfac2 | Oracle ID: da350f46-6d6e-4257-8f4f-77769c442d0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRITUAL_FOCUS,
    oracle_id = "da350f46-6d6e-4257-8f4f-77769c442d0c",
    scryfall_id = "8521ae08-eb46-45ff-8fc4-62d8b07cfac2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spiritual Focus",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
