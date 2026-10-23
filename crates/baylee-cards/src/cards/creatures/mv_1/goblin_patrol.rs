//! Goblin Patrol — {R} — Creature — Goblin
//! Oracle: Echo {R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #193 — Urza's Saga | Scryfall ID: d0fcd8d3-f159-49a1-8dd9-582ae4a0adc3 | Oracle ID: edf25cbe-fa28-44ca-b4be-1d2312e4f6ac
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_PATROL,
    oracle_id = "edf25cbe-fa28-44ca-b4be-1d2312e4f6ac",
    scryfall_id = "d0fcd8d3-f159-49a1-8dd9-582ae4a0adc3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Patrol",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[AbilityDef::Echo { cost: mana!("{R}") }],
);
