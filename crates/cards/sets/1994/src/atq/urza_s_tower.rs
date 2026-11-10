//! Urza's Tower — (no cost) — Land — Urza's Tower
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.
//! Set: CMM #1053 — Commander Masters | Scryfall ID: 1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752 | Oracle ID: 32fbb638-ab14-4e8b-a07a-d4c44e3496f2

use crate::filters::{URZA_S_MINE, URZA_S_POWER_PLANT};
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_TOWER,
    oracle_id = "32fbb638-ab14-4e8b-a07a-d4c44e3496f2",
    scryfall_id = "1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752",
    faces = &[face!(
        name = "Urza's Tower",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::TOWER],
    ),],
    coverage = Coverage::Implemented,
    // "If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C}
    // instead." The {C} is always made, and "instead" is the two more it takes
    // to reach the larger amount: a mana ability makes its mana at the top of
    // its list, where `lints::mana_ability_fault` looks, and the "and" is two
    // nested conditions, one per land type.
    abilities = &[mana_ability!(&[
        Effect::mana(ManaColor::Colorless, 1),
        Effect::IfCondition {
            condition: Condition::ControlCount(&URZA_S_MINE, 1),
            then: &[Effect::IfCondition {
                condition: Condition::ControlCount(&URZA_S_POWER_PLANT, 1),
                then: &[Effect::mana(ManaColor::Colorless, 2)],
                otherwise: &[],
            }],
            otherwise: &[],
        },
    ])],
);
