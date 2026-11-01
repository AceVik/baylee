//! Backlash — {1}{B}{R} — Instant
//! Oracle: Tap target untapped creature. That creature deals damage equal to its power to its controller.
//! Set: INV #234 — Invasion | Scryfall ID: dadf030d-5451-43fc-bf0c-c1629fdf88ec | Oracle ID: 3db9bf7b-da4e-428f-aa5d-a0cd1afeb16c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BACKLASH,
    oracle_id = "3db9bf7b-da4e-428f-aa5d-a0cd1afeb16c",
    scryfall_id = "dadf030d-5451-43fc-bf0c-c1629fdf88ec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Backlash",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
