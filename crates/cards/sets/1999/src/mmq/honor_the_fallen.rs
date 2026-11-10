//! Honor the Fallen — {1}{W} — Instant
//! Oracle: Exile all creature cards from all graveyards. You gain 1 life for each card exiled this way.
//! Set: MMQ #21 — Mercadian Masques | Scryfall ID: 70147617-10e0-413d-be0a-a888b9cb6b97 | Oracle ID: db557d5e-eecd-4891-90fa-c5bcb8532015
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HONOR_THE_FALLEN,
    oracle_id = "db557d5e-eecd-4891-90fa-c5bcb8532015",
    scryfall_id = "70147617-10e0-413d-be0a-a888b9cb6b97",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Honor the Fallen",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
