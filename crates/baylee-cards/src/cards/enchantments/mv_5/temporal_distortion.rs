//! Temporal Distortion — {3}{U}{U} — Enchantment
//! Oracle: Whenever a creature or land becomes tapped, put an hourglass counter on it.
//! Oracle: Each permanent with an hourglass counter on it doesn't untap during its controller's untap step.
//! Oracle: At the beginning of each player's upkeep, remove all hourglass counters from permanents that player controls.
//! Set: INV #79 — Invasion | Scryfall ID: 74bd0d14-8d26-403f-9405-d0dcdecd1a49 | Oracle ID: b0a16783-31c2-4b2a-835d-774dc385f99a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPORAL_DISTORTION,
    oracle_id = "b0a16783-31c2-4b2a-835d-774dc385f99a",
    scryfall_id = "74bd0d14-8d26-403f-9405-d0dcdecd1a49",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Temporal Distortion",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
