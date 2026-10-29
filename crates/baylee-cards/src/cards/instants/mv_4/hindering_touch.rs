//! Hindering Touch — {3}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {2}.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: SCG #37 — Scourge | Scryfall ID: db9735d9-4aac-4175-8ec8-fc9bfd8f2c5c | Oracle ID: 90c6f338-fc41-4499-8f39-1cd202a2cd31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HINDERING_TOUCH,
    oracle_id = "90c6f338-fc41-4499-8f39-1cd202a2cd31",
    scryfall_id = "db9735d9-4aac-4175-8ec8-fc9bfd8f2c5c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hindering Touch",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
