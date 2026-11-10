//! Urza's Power Plant — (no cost) — Land — Urza's Power-Plant
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1052 — Commander Masters | Scryfall ID: b0449a19-37f7-4169-9e32-928db5ec76fe | Oracle ID: e11966cd-2ee3-4df4-b099-abf42dcdf0db

use crate::filters::{URZA_S_MINE, URZA_S_TOWER};
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_POWER_PLANT,
    oracle_id = "e11966cd-2ee3-4df4-b099-abf42dcdf0db",
    scryfall_id = "b0449a19-37f7-4169-9e32-928db5ec76fe",
    faces = &[face!(
        name = "Urza's Power Plant",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::POWER_PLANT],
    ),],
    coverage = Coverage::Implemented,
    // "If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead."
    // The {C} is always made, and "instead" is the one more it takes to reach
    // the larger amount: a mana ability makes its mana at the top of its list,
    // where `lints::mana_ability_fault` looks, and the "and" is two nested
    // conditions, one per land type.
    abilities = &[mana_ability!(&[
        Effect::mana(ManaColor::Colorless, 1),
        Effect::IfCondition {
            condition: Condition::ControlCount(&URZA_S_MINE, 1),
            then: &[Effect::IfCondition {
                condition: Condition::ControlCount(&URZA_S_TOWER, 1),
                then: &[Effect::mana(ManaColor::Colorless, 1)],
                otherwise: &[],
            }],
            otherwise: &[],
        },
    ])],
);
