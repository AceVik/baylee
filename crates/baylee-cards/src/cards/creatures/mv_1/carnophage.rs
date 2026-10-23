//! Carnophage — {B} — Creature — Zombie
//! Oracle: At the beginning of your upkeep, tap this creature unless you pay 1 life.
//! Set: TPR #84 — Tempest Remastered | Scryfall ID: 889a92ca-e198-494d-9ccb-6ae2c19ec62e | Oracle ID: 9ac77cfd-7d78-4c8a-bffc-17649a852114
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARNOPHAGE,
    oracle_id = "9ac77cfd-7d78-4c8a-bffc-17649a852114",
    scryfall_id = "889a92ca-e198-494d-9ccb-6ae2c19ec62e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Carnophage",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::You,
            life: Amount::Fixed(1),
            effect: &Effect::TapAll {
                filter: &Filter::This,
            },
        }],
    )],
);
