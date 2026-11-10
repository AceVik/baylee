//! Parallel Thoughts — {3}{U}{U} — Enchantment
//! Oracle: When this enchantment enters, search your library for seven cards, exile them in a face-down pile, and shuffle that pile. Then shuffle your library.
//! Oracle: If you would draw a card, you may instead put the top card of the pile you exiled into your hand.
//! Set: SCG #44 — Scourge | Scryfall ID: d913c541-a8fb-4383-bbab-988be3e0f5d5 | Oracle ID: 2daee7cb-6736-4aa6-9921-0371c1abe69f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PARALLEL_THOUGHTS,
    oracle_id = "2daee7cb-6736-4aa6-9921-0371c1abe69f",
    scryfall_id = "d913c541-a8fb-4383-bbab-988be3e0f5d5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Parallel Thoughts",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
