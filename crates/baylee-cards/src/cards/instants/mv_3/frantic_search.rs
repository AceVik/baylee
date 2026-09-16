//! Frantic Search — {2}{U} — Instant
//! Oracle: Draw two cards, then discard two cards. Untap up to three lands.
//! Set: TLE #159 — Avatar: The Last Airbender Eternal | Scryfall ID: d8c5e52d-57ae-464b-8860-7dd39ebfef64 | Oracle ID: 16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FRANTIC_SEARCH,
    oracle_id = "16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c",
    scryfall_id = "d8c5e52d-57ae-464b-8860-7dd39ebfef64",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Frantic Search",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
