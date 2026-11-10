//! Story Circle — {1}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a color.
//! Oracle: {W}: The next time a source of your choice of the chosen color would deal damage to you this turn, prevent that damage.
//! Set: 10E #49 — Tenth Edition | Scryfall ID: 6194fab1-c185-4df5-8900-37d28ffae545 | Oracle ID: 7071aee8-b5ca-4be5-9ba0-2df7e3af303b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORY_CIRCLE,
    oracle_id = "7071aee8-b5ca-4be5-9ba0-2df7e3af303b",
    scryfall_id = "6194fab1-c185-4df5-8900-37d28ffae545",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Story Circle",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
