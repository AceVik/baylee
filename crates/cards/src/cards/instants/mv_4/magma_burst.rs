//! Magma Burst — {3}{R} — Instant
//! Oracle: Kicker—Sacrifice two lands. (You may sacrifice two lands in addition to any other costs as you cast this spell.)
//! Oracle: Magma Burst deals 3 damage to any target. If this spell was kicked, it deals 3 damage to another target.
//! Set: PLS #66 — Planeshift | Scryfall ID: d9752bc3-0bdf-4657-8750-73c8cbc8e83f | Oracle ID: f5af6d60-decd-4e04-8841-da2d3000a405
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAGMA_BURST,
    oracle_id = "f5af6d60-decd-4e04-8841-da2d3000a405",
    scryfall_id = "d9752bc3-0bdf-4657-8750-73c8cbc8e83f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Magma Burst",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
