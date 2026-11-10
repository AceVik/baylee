//! Bomb Squad — {3}{R} — Creature — Dwarf
//! Oracle: {T}: Put a fuse counter on target creature.
//! Oracle: At the beginning of your upkeep, put a fuse counter on each creature with a fuse counter on it.
//! Oracle: Whenever a creature has four or more fuse counters on it, remove all fuse counters from it and destroy it. That creature deals 4 damage to its controller.
//! Set: ODY #179 — Odyssey | Scryfall ID: 8e9535a5-29ea-4085-a36b-4905d85e97ac | Oracle ID: f2ecb354-8f79-4c39-989e-7aa37ec75154
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BOMB_SQUAD,
    oracle_id = "f2ecb354-8f79-4c39-989e-7aa37ec75154",
    scryfall_id = "8e9535a5-29ea-4085-a36b-4905d85e97ac",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Bomb Squad",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
