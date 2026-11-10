//! Peek — {U} — Instant
//! Oracle: Look at target player's hand.
//! Oracle: Draw a card.
//! Set: 10E #94 — Tenth Edition | Scryfall ID: 869a6c06-eae5-418e-9a5c-26598a929416 | Oracle ID: 873abbf4-c9b5-4b8f-8cd0-613cf3b9b1d5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PEEK,
    oracle_id = "873abbf4-c9b5-4b8f-8cd0-613cf3b9b1d5",
    scryfall_id = "869a6c06-eae5-418e-9a5c-26598a929416",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Peek",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
