//! Scattershot — {2}{R} — Instant
//! Oracle: Scattershot deals 1 damage to target creature.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: SCG #102 — Scourge | Scryfall ID: cf22f3e7-1626-4bab-9f62-7d4774704395 | Oracle ID: ed3652da-0127-4c9e-9913-f20becdd0ba9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCATTERSHOT,
    oracle_id = "ed3652da-0127-4c9e-9913-f20becdd0ba9",
    scryfall_id = "cf22f3e7-1626-4bab-9f62-7d4774704395",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scattershot",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
