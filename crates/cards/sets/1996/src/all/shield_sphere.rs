//! Shield Sphere — {0} — Artifact Creature — Wall
//! Oracle: Defender
//! Oracle: Whenever this creature blocks, put a -0/-1 counter on it.
//! Set: ME1 #166 — Masters Edition | Scryfall ID: 7a76a97e-a857-4593-ae7c-e393d8a868ac | Oracle ID: df9ec9b0-be22-4136-923b-11a71ca4b6c2
// IMPLEMENTED — defender; a -0/-1 counter on it each time it blocks.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIELD_SPHERE,
    oracle_id = "df9ec9b0-be22-4136-923b-11a71ca4b6c2",
    scryfall_id = "7a76a97e-a857-4593-ae7c-e393d8a868ac",
    faces = &[face!(
        name = "Shield Sphere",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        keywords = KeywordSet::DEFENDER,
        power = Some(0),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::AddCounterSource {
            kind: CounterKind::Minus {
                power: 0,
                toughness: 1,
            },
            amount: Amount::Fixed(1),
        }]
    )],
);
