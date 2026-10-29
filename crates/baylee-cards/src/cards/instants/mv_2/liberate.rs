//! Liberate — {1}{W} — Instant
//! Oracle: Exile target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.
//! Set: INV #21 — Invasion | Scryfall ID: 96794470-31ea-478f-b11c-dc8342a508e2 | Oracle ID: f17e6c85-4813-4b0f-a0e9-d295822e9bb5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIBERATE,
    oracle_id = "f17e6c85-4813-4b0f-a0e9-d295822e9bb5",
    scryfall_id = "96794470-31ea-478f-b11c-dc8342a508e2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Liberate",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
