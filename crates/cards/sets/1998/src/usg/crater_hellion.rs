//! Crater Hellion — {4}{R}{R} — Creature — Hellion Beast
//! Oracle: Echo {4}{R}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, it deals 4 damage to each other creature.
//! Set: USG #179 — Urza's Saga | Scryfall ID: 2382e525-1750-484a-bf95-dbb42bbb30ae | Oracle ID: 2d37c437-0d5c-400d-88ce-10d173b28eda
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRATER_HELLION,
    oracle_id = "2d37c437-0d5c-400d-88ce-10d173b28eda",
    scryfall_id = "2382e525-1750-484a-bf95-dbb42bbb30ae",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crater Hellion",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HELLION, subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{4}{R}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::DealDamageEach {
                amount: Amount::Fixed(4),
                filter: &Filter::And(&[Filter::CREATURE, Filter::Another]),
            }],
        ),
    ],
);
