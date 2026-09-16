//! Abrupt Decay — {B}{G} — Instant
//! Oracle: This spell can't be countered.
//! Oracle: Destroy target nonland permanent with mana value 3 or less.
//! Set: MM3 #146 — Modern Masters 2017 | Scryfall ID: a8e328c6-3a84-49cf-a1a3-1d1e5373d274 | Oracle ID: 1c747fe2-289e-492a-a846-aa77707e2dc3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABRUPT_DECAY,
    oracle_id = "1c747fe2-289e-492a-a846-aa77707e2dc3",
    scryfall_id = "a8e328c6-3a84-49cf-a1a3-1d1e5373d274",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Abrupt Decay",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
