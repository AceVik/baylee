//! Circle of Protection: Green — {1}{W} — Enchantment
//! Oracle: {1}: The next time a green source of your choice would deal damage to you this turn, prevent that damage.
//! Set: 8ED #12 — Eighth Edition | Scryfall ID: eecc0e7d-a15b-4930-93d1-832c4fed733b | Oracle ID: 41b0f347-1398-4778-bf3f-4007d8a77162
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCLE_OF_PROTECTION_GREEN,
    oracle_id = "41b0f347-1398-4778-bf3f-4007d8a77162",
    scryfall_id = "eecc0e7d-a15b-4930-93d1-832c4fed733b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Circle of Protection: Green",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
