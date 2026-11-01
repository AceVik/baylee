//! Sacred Ground — {1}{W} — Enchantment
//! Oracle: Whenever a spell or ability an opponent controls causes a land to be put into your graveyard from the battlefield, return that card to the battlefield.
//! Set: 9ED #37 — Ninth Edition | Scryfall ID: d72dc68a-d6f9-4a18-8282-0f104999591f | Oracle ID: b18e773c-611b-4c18-8b2d-8d3a7e5ddc93
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SACRED_GROUND,
    oracle_id = "b18e773c-611b-4c18-8b2d-8d3a7e5ddc93",
    scryfall_id = "d72dc68a-d6f9-4a18-8282-0f104999591f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sacred Ground",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
