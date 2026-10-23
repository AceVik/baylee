//! Electryte — {3}{R}{R} — Creature — Trilobite Beast
//! Oracle: Whenever this creature deals combat damage to defending player, it deals damage equal to its power to each blocking creature.
//! Set: USG #183 — Urza's Saga | Scryfall ID: 85c3d04f-4010-4db3-9e4e-afa8116b263d | Oracle ID: 5ff0c25f-4593-460b-9d7f-d9c37f135a18
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Blocking]);

card!(
    index = index::ELECTRYTE,
    oracle_id = "5ff0c25f-4593-460b-9d7f-d9c37f135a18",
    scryfall_id = "85c3d04f-4010-4db3-9e4e-afa8116b263d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Electryte",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TRILOBITE, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::DealDamageEach {
            amount: Amount::SourcePower,
            filter: &BLOCKING_CREATURE,
        }],
    )],
);
