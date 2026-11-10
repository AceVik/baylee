//! Backslide — {1}{U} — Instant
//! Oracle: Turn target creature with a morph ability face down.
//! Oracle: Cycling {U} ({U}, Discard this card: Draw a card.)
//! Set: ONS #70 — Onslaught | Scryfall ID: 47c40269-80a5-454f-83dd-dae1c11500c0 | Oracle ID: 6d9746bc-0b72-4ac4-b48a-742b63b1c41b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BACKSLIDE,
    oracle_id = "6d9746bc-0b72-4ac4-b48a-742b63b1c41b",
    scryfall_id = "47c40269-80a5-454f-83dd-dae1c11500c0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Backslide",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
