//! Blessed Wind — {7}{W}{W} — Sorcery
//! Oracle: Target player's life total becomes 20.
//! Set: PCY #4 — Prophecy | Scryfall ID: 3cb624d6-9aec-498c-8df9-6fd025c74487 | Oracle ID: 93da4a35-d2d8-476c-ad03-318101c0118b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLESSED_WIND,
    oracle_id = "93da4a35-d2d8-476c-ad03-318101c0118b",
    scryfall_id = "3cb624d6-9aec-498c-8df9-6fd025c74487",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Blessed Wind",
        mana_cost = mana!("{7}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
