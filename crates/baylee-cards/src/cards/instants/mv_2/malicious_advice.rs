//! Malicious Advice — {X}{U}{B} — Instant
//! Oracle: Tap X target artifacts, creatures, and/or lands. You lose X life.
//! Set: PLS #114 — Planeshift | Scryfall ID: 7b1547c2-ae9f-4871-a675-4026bf20e7e1 | Oracle ID: 14c1adf8-950b-4f64-9c84-956a623448f4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MALICIOUS_ADVICE,
    oracle_id = "14c1adf8-950b-4f64-9c84-956a623448f4",
    scryfall_id = "7b1547c2-ae9f-4871-a675-4026bf20e7e1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Malicious Advice",
        mana_cost = mana!("{X}{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
