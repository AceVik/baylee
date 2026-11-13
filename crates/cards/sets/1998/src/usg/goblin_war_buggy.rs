//! Goblin War Buggy — {1}{R} — Creature — Goblin
//! Oracle: Haste
//! Oracle: Echo {1}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #196 — Urza's Saga | Scryfall ID: d2d0fc9e-fb6b-4a00-b422-32565f7ce454 | Oracle ID: 59fb1640-d8ed-476a-a78d-91d60b6faac5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_WAR_BUGGY,
    oracle_id = "59fb1640-d8ed-476a-a78d-91d60b6faac5",
    scryfall_id = "d2d0fc9e-fb6b-4a00-b422-32565f7ce454",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin War Buggy",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{1}{R}"),
    }],
);
