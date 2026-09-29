//! Ironclaw Orcs — {1}{R} — Creature — Orc
//! Oracle: This creature can't block creatures with power 2 or greater.
//! Set: ME2 #132 — Masters Edition II | Scryfall ID: abee9c59-48c2-4c2c-a3d6-ccb0d4652831 | Oracle ID: c645a616-0d7d-416c-b5f3-057b3a1666a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IRONCLAW_ORCS,
    oracle_id = "c645a616-0d7d-416c-b5f3-057b3a1666a0",
    scryfall_id = "abee9c59-48c2-4c2c-a3d6-ccb0d4652831",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ironclaw Orcs",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
