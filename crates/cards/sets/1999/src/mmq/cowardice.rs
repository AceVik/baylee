//! Cowardice — {3}{U}{U} — Enchantment
//! Oracle: Whenever a creature becomes the target of a spell or ability, return that creature to its owner's hand. (It won't be affected by the spell or ability.)
//! Set: 9ED #70 — Ninth Edition | Scryfall ID: 67c3cf8f-c2f6-4ad0-83f9-960e5d74fc3e | Oracle ID: 2692dd66-c200-4d0b-9427-30deaa2342ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COWARDICE,
    oracle_id = "2692dd66-c200-4d0b-9427-30deaa2342ab",
    scryfall_id = "67c3cf8f-c2f6-4ad0-83f9-960e5d74fc3e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cowardice",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
