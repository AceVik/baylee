//! Repel — {3}{U} — Instant
//! Oracle: Put target creature on top of its owner's library.
//! Set: VMA #89 — Vintage Masters | Scryfall ID: 81cf7987-e478-4a76-8f5e-ef9c1bb3b30a | Oracle ID: 6e072b3a-7a83-4d77-bae3-72de0e646b81
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPEL,
    oracle_id = "6e072b3a-7a83-4d77-bae3-72de0e646b81",
    scryfall_id = "81cf7987-e478-4a76-8f5e-ef9c1bb3b30a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Repel",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
