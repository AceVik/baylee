//! Mogg War Marshal — {1}{R} — Creature — Goblin Warrior
//! Oracle: Echo {1}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters or dies, create a 1/1 red Goblin creature token.
//! Set: PLST #DMR-131 — The List | Scryfall ID: 4ebce261-3a53-48f2-a173-0d98c68aff2c | Oracle ID: 7b8f4879-578e-40e4-863a-41d0c32c6bdd
// IMPLEMENTED — echo; entering or dying creates one 1/1 red Goblin token.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_WAR_MARSHAL,
    oracle_id = "7b8f4879-578e-40e4-863a-41d0c32c6bdd",
    scryfall_id = "4ebce261-3a53-48f2-a173-0d98c68aff2c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mogg War Marshal",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{1}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::CreateToken {
                token: &generated_tokens::GOBLIN_1_1_RED,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::CreateToken {
                token: &generated_tokens::GOBLIN_1_1_RED,
            }],
        ),
    ],
);
