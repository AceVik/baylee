//! Cinder Giant — {3}{R} — Creature — Giant
//! Oracle: At the beginning of your upkeep, this creature deals 2 damage to each other creature you control.
//! Set: WTH #93 — Weatherlight | Scryfall ID: de97c939-2c44-4c43-9d66-1087bcee692b | Oracle ID: 3a8306ae-3162-4105-95f4-dacd10c7c051
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CINDER_GIANT,
    oracle_id = "3a8306ae-3162-4105-95f4-dacd10c7c051",
    scryfall_id = "de97c939-2c44-4c43-9d66-1087bcee692b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cinder Giant",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::damage_each(
            2,
            &Filter::ANOTHER_CREATURE_YOU_CONTROL
        )],
    )],
);
