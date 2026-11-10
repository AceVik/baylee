//! Renounce — {1}{W} — Instant
//! Oracle: Sacrifice any number of permanents. You gain 2 life for each permanent sacrificed this way.
//! Set: MMQ #42 — Mercadian Masques | Scryfall ID: 8bb2bfb9-cc4a-4d33-99f9-17db4d9fc718 | Oracle ID: 72f9a6f7-ea26-438d-aafa-758239fd3681
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RENOUNCE,
    oracle_id = "72f9a6f7-ea26-438d-aafa-758239fd3681",
    scryfall_id = "8bb2bfb9-cc4a-4d33-99f9-17db4d9fc718",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Renounce",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
