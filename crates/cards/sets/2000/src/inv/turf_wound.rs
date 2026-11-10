//! Turf Wound — {2}{R} — Instant
//! Oracle: Target player can't play lands this turn.
//! Oracle: Draw a card.
//! Set: INV #177 — Invasion | Scryfall ID: 91392e9f-f96a-4ac5-b1f1-c73540cf249e | Oracle ID: 8dc48180-1482-40c4-8c5f-1724018e9c5b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TURF_WOUND,
    oracle_id = "8dc48180-1482-40c4-8c5f-1724018e9c5b",
    scryfall_id = "91392e9f-f96a-4ac5-b1f1-c73540cf249e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Turf Wound",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
