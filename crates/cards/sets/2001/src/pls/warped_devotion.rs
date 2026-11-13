//! Warped Devotion — {2}{B} — Enchantment
//! Oracle: Whenever a permanent is returned to a player's hand, that player discards a card.
//! Set: 8ED #172 — Eighth Edition | Scryfall ID: ffd41ebd-91a8-4972-8f93-3306e68519f9 | Oracle ID: 22c49415-e8d2-4b4c-b021-57ee54670f86
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WARPED_DEVOTION,
    oracle_id = "22c49415-e8d2-4b4c-b021-57ee54670f86",
    scryfall_id = "ffd41ebd-91a8-4972-8f93-3306e68519f9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Warped Devotion",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
