//! Veteran Brawlers — {1}{R} — Creature — Human Soldier
//! Oracle: This creature can't attack if defending player controls an untapped land.
//! Oracle: This creature can't block if you control an untapped land.
//! Set: PCY #106 — Prophecy | Scryfall ID: ee4d3acb-68be-409d-beb7-92a7cbc0402f | Oracle ID: 77a963a3-473a-4c34-bc46-8ef016a92c0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VETERAN_BRAWLERS,
    oracle_id = "77a963a3-473a-4c34-bc46-8ef016a92c0c",
    scryfall_id = "ee4d3acb-68be-409d-beb7-92a7cbc0402f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Veteran Brawlers",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
