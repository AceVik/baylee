//! Unified Strike — {W} — Instant
//! Oracle: Exile target attacking creature if its power is less than or equal to the number of Soldiers on the battlefield.
//! Set: ONS #58 — Onslaught | Scryfall ID: 29906eca-0823-4cd6-890f-e5b93cc50a11 | Oracle ID: 4649991c-4f2c-49c8-9e18-d0ae36328553
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNIFIED_STRIKE,
    oracle_id = "4649991c-4f2c-49c8-9e18-d0ae36328553",
    scryfall_id = "29906eca-0823-4cd6-890f-e5b93cc50a11",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Unified Strike",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
