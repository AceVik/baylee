//! Moment of Silence — {W} — Instant
//! Oracle: Target player skips their next combat phase this turn.
//! Set: MMQ #28 — Mercadian Masques | Scryfall ID: 3f50b5f5-8dac-4785-b1af-a0bd64ce7a92 | Oracle ID: 43af5041-a4bd-4b61-b1a1-bdea916afc59
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOMENT_OF_SILENCE,
    oracle_id = "43af5041-a4bd-4b61-b1a1-bdea916afc59",
    scryfall_id = "3f50b5f5-8dac-4785-b1af-a0bd64ce7a92",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Moment of Silence",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
