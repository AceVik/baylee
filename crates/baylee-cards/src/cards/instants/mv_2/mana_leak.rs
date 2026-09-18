//! Mana Leak — {1}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {3}.
//! Set: 2X2 #58 — Double Masters 2022 | Scryfall ID: 179236d9-6fe2-4db6-bdfb-f851e8d531a2 | Oracle ID: c61fe162-2202-4e56-9ba0-393547f9875f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_LEAK,
    oracle_id = "c61fe162-2202-4e56-9ba0-393547f9875f",
    scryfall_id = "179236d9-6fe2-4db6-bdfb-f851e8d531a2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mana Leak",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
