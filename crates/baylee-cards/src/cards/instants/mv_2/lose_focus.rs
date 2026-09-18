//! Lose Focus — {1}{U} — Instant
//! Oracle: Replicate {U} (When you cast this spell, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)
//! Oracle: Counter target spell unless its controller pays {2}.
//! Set: MH2 #49 — Modern Horizons 2 | Scryfall ID: 985bdb0c-ce6c-4506-8163-76f3b2fdf5fb | Oracle ID: 1cea6439-7ae5-4887-8c33-7da9fb36e2d4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOSE_FOCUS,
    oracle_id = "1cea6439-7ae5-4887-8c33-7da9fb36e2d4",
    scryfall_id = "985bdb0c-ce6c-4506-8163-76f3b2fdf5fb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lose Focus",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
