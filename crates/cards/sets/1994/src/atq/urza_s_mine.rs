//! Urza's Mine — (no cost) — Land — Urza's Mine
//! Oracle: {T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1051 — Commander Masters | Scryfall ID: 396bbb7d-ae61-4d8d-b931-9ed2f712832e | Oracle ID: 33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0

use crate::filters::{URZA_S_POWER_PLANT, URZA_S_TOWER};
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_MINE,
    oracle_id = "33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0",
    scryfall_id = "396bbb7d-ae61-4d8d-b931-9ed2f712832e",
    faces = &[face!(
        name = "Urza's Mine",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::MINE],
    ),],
    coverage = Coverage::Implemented,
    // "If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C}
    // instead." The {C} is always made, and "instead" is the one more it takes
    // to reach the larger amount: a mana ability makes its mana at the top of
    // its list, where `lints::mana_ability_fault` looks, and the "and" is two
    // nested conditions, one per land type.
    abilities = &[mana_ability!(&[
        Effect::mana(ManaColor::Colorless, 1),
        Effect::IfCondition {
            condition: Condition::ControlCount(&URZA_S_POWER_PLANT, 1),
            then: &[Effect::IfCondition {
                condition: Condition::ControlCount(&URZA_S_TOWER, 1),
                then: &[Effect::mana(ManaColor::Colorless, 1)],
                otherwise: &[],
            }],
            otherwise: &[],
        },
    ])],
);
