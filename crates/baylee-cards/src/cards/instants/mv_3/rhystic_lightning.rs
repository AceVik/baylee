//! Rhystic Lightning — {2}{R} — Instant
//! Oracle: Rhystic Lightning deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, Rhystic Lightning deals 2 damage to the permanent or player.
//! Set: PCY #99 — Prophecy | Scryfall ID: 21ce365e-3002-42e9-aeb5-1b845408271e | Oracle ID: f6d371e6-e25e-4820-8cf7-f692a4d9be11
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_LIGHTNING,
    oracle_id = "f6d371e6-e25e-4820-8cf7-f692a4d9be11",
    scryfall_id = "21ce365e-3002-42e9-aeb5-1b845408271e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rhystic Lightning",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
