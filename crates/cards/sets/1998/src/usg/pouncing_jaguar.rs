//! Pouncing Jaguar — {G} — Creature — Cat
//! Oracle: Echo {G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #269 — Urza's Saga | Scryfall ID: d35ac6e5-3e46-4290-9683-51d6f54e4edf | Oracle ID: 70a2b5ea-dae4-46e8-87eb-8c8d8dfb242b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POUNCING_JAGUAR,
    oracle_id = "70a2b5ea-dae4-46e8-87eb-8c8d8dfb242b",
    scryfall_id = "d35ac6e5-3e46-4290-9683-51d6f54e4edf",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pouncing Jaguar",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[AbilityDef::Echo { cost: mana!("{G}") },],
);
