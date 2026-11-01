//! Goblin Marshal — {4}{R}{R} — Creature — Goblin Warrior
//! Oracle: Echo {4}{R}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters or dies, create two 1/1 red Goblin creature tokens.
//! Set: UDS #85 — Urza's Destiny | Scryfall ID: 6a85b2f9-c12c-46dd-ae04-470ebf5ec6d9 | Oracle ID: 4bc91d7d-37aa-475c-8fe9-763975d51add
// IMPLEMENTED — echo {4}{R}{R}; entering the battlefield or dying each creates two 1/1 red Goblin creature tokens.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_MARSHAL,
    oracle_id = "4bc91d7d-37aa-475c-8fe9-763975d51add",
    scryfall_id = "6a85b2f9-c12c-46dd-ae04-470ebf5ec6d9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Marshal",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{4}{R}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[
                Effect::CreateToken {
                    token: &generated_tokens::GOBLIN_1_1_RED,
                },
                Effect::CreateToken {
                    token: &generated_tokens::GOBLIN_1_1_RED,
                },
            ],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[
                Effect::CreateToken {
                    token: &generated_tokens::GOBLIN_1_1_RED,
                },
                Effect::CreateToken {
                    token: &generated_tokens::GOBLIN_1_1_RED,
                },
            ],
        ),
    ],
);
