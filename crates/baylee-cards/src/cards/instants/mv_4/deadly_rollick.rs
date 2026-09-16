//! Deadly Rollick — {3}{B} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Exile target creature.
//! Set: CMM #147 — Commander Masters | Scryfall ID: 0e13f735-54fa-42b6-aea4-ced33811d7d4 | Oracle ID: 0456ec64-2c81-4763-a352-8ff64a4c3d6b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEADLY_ROLLICK,
    oracle_id = "0456ec64-2c81-4763-a352-8ff64a4c3d6b",
    scryfall_id = "0e13f735-54fa-42b6-aea4-ced33811d7d4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Deadly Rollick",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
