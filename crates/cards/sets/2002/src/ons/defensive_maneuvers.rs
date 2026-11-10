//! Defensive Maneuvers — {3}{W} — Instant
//! Oracle: Creatures of the creature type of your choice get +0/+4 until end of turn.
//! Set: ONS #23 — Onslaught | Scryfall ID: 58f9eb25-4140-4ecf-bcaa-1b193d884007 | Oracle ID: 812701c3-c9b7-4877-bf2e-c37e0f551c45
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFENSIVE_MANEUVERS,
    oracle_id = "812701c3-c9b7-4877-bf2e-c37e0f551c45",
    scryfall_id = "58f9eb25-4140-4ecf-bcaa-1b193d884007",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Defensive Maneuvers",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
