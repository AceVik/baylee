//! Mine Layer — {3}{R} — Creature — Dwarf
//! Oracle: {1}{R}, {T}: Put a mine counter on target land.
//! Oracle: Whenever a land with a mine counter on it becomes tapped, destroy it.
//! Oracle: When this creature leaves the battlefield, remove all mine counters from all lands.
//! Set: ODY #205 — Odyssey | Scryfall ID: 497efda1-5013-45d2-b717-b0296218c42b | Oracle ID: 7cf29c48-ba07-4af3-b846-7f81046f4059
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MINE_LAYER,
    oracle_id = "7cf29c48-ba07-4af3-b846-7f81046f4059",
    scryfall_id = "497efda1-5013-45d2-b717-b0296218c42b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mine Layer",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
