//! Circle of Protection: Blue — {1}{W} — Enchantment
//! Oracle: {1}: The next time a blue source of your choice would deal damage to you this turn, prevent that damage.
//! Set: 8ED #11 — Eighth Edition | Scryfall ID: 5ced2118-dfb3-4f29-ad6b-454c0a8a094b | Oracle ID: d7572f17-f85d-45c0-ac64-43aac760eafe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_PROTECTION_BLUE,
    oracle_id = "d7572f17-f85d-45c0-ac64-43aac760eafe",
    scryfall_id = "5ced2118-dfb3-4f29-ad6b-454c0a8a094b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Circle of Protection: Blue",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
