//! Forbidden Crypt — {3}{B}{B} — Enchantment
//! Oracle: If you would draw a card, return a card from your graveyard to your hand instead. If you can't, you lose the game.
//! Oracle: If a card would be put into your graveyard from anywhere, exile that card instead.
//! Set: 6ED #132 — Classic Sixth Edition | Scryfall ID: 76f5f509-707b-4ed6-9824-626dea5869b0 | Oracle ID: 8b34b211-985b-4934-bb3b-8e6672685ac2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORBIDDEN_CRYPT,
    oracle_id = "8b34b211-985b-4934-bb3b-8e6672685ac2",
    scryfall_id = "76f5f509-707b-4ed6-9824-626dea5869b0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Forbidden Crypt",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
