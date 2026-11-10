//! Divert — {U} — Instant
//! Oracle: Change the target of target spell with a single target unless that spell's controller pays {2}.
//! Set: ODY #82 — Odyssey | Scryfall ID: 8eb6e9f8-a508-451e-8a72-5f9834ba5352 | Oracle ID: 561aed01-322b-4699-b7b0-f4416075aa9b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVERT,
    oracle_id = "561aed01-322b-4699-b7b0-f4416075aa9b",
    scryfall_id = "8eb6e9f8-a508-451e-8a72-5f9834ba5352",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Divert",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
