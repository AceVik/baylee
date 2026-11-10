//! Break Open — {1}{R} — Instant
//! Oracle: Turn target face-down creature an opponent controls face up.
//! Set: ONS #190 — Onslaught | Scryfall ID: a5ae8050-b644-41db-b1e9-d9bad2173485 | Oracle ID: 1fcec3b2-47ba-4c56-9ce1-a4807e8ad262
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREAK_OPEN,
    oracle_id = "1fcec3b2-47ba-4c56-9ce1-a4807e8ad262",
    scryfall_id = "a5ae8050-b644-41db-b1e9-d9bad2173485",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Break Open",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
