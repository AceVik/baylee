//! Aven Farseer — {1}{W} — Creature — Bird Soldier
//! Oracle: Flying
//! Oracle: Whenever a permanent is turned face up, put a +1/+1 counter on this creature.
//! Set: SCG #3 — Scourge | Scryfall ID: 47854e89-4d22-4eb6-a77d-6f04407bd2e5 | Oracle ID: 4591e1e5-5cac-4812-80e0-67c1237862d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_FARSEER,
    oracle_id = "4591e1e5-5cac-4812-80e0-67c1237862d3",
    scryfall_id = "47854e89-4d22-4eb6-a77d-6f04407bd2e5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Farseer",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
