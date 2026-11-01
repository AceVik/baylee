//! Suffocating Blast — {1}{U}{U}{R} — Instant
//! Oracle: Counter target spell and Suffocating Blast deals 3 damage to target creature.
//! Set: APC #124 — Apocalypse | Scryfall ID: c2a70297-2a7b-4a0c-ace5-cd61bfe6dafd | Oracle ID: 2a9d9433-7b0f-45a3-88aa-9fd7e410c224
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUFFOCATING_BLAST,
    oracle_id = "2a9d9433-7b0f-45a3-88aa-9fd7e410c224",
    scryfall_id = "c2a70297-2a7b-4a0c-ace5-cd61bfe6dafd",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Suffocating Blast",
        mana_cost = mana!("{1}{U}{U}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
