//! Thermal Blast — {4}{R} — Instant
//! Oracle: Thermal Blast deals 3 damage to target creature.
//! Oracle: Threshold — Thermal Blast deals 5 damage instead if there are seven or more cards in your graveyard.
//! Set: ODY #224 — Odyssey | Scryfall ID: 623cc141-0f75-4e67-b852-6c144d98e619 | Oracle ID: 52ad1a31-3ad8-4ead-ba77-675cad7e4a14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THERMAL_BLAST,
    oracle_id = "52ad1a31-3ad8-4ead-ba77-675cad7e4a14",
    scryfall_id = "623cc141-0f75-4e67-b852-6c144d98e619",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Thermal Blast",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
