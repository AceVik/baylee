//! Dueling Grounds — {1}{G}{W} — Enchantment
//! Oracle: No more than one creature can attack each combat.
//! Oracle: No more than one creature can block each combat.
//! Set: INV #245 — Invasion | Scryfall ID: 52760183-bee0-4ce0-96c0-074b88f78980 | Oracle ID: eb2df4a1-6b63-4f6a-a830-e9487afe59f8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUELING_GROUNDS,
    oracle_id = "eb2df4a1-6b63-4f6a-a830-e9487afe59f8",
    scryfall_id = "52760183-bee0-4ce0-96c0-074b88f78980",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Dueling Grounds",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
