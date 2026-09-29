//! Circle of Protection: White — {1}{W} — Enchantment
//! Oracle: {1}: The next time a white source of your choice would deal damage to you this turn, prevent that damage.
//! Set: 8ED #14 — Eighth Edition | Scryfall ID: 2e64f2cc-67bd-482a-938b-8dca1a1c6ac1 | Oracle ID: 5f46f86a-9779-4ed3-99cb-76a03d380598
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_PROTECTION_WHITE,
    oracle_id = "5f46f86a-9779-4ed3-99cb-76a03d380598",
    scryfall_id = "2e64f2cc-67bd-482a-938b-8dca1a1c6ac1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Circle of Protection: White",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
