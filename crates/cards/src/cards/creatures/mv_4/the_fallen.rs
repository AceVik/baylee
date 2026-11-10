//! The Fallen — {1}{B}{B}{B} — Creature — Zombie
//! Oracle: At the beginning of your upkeep, this creature deals 1 damage to each opponent and planeswalker it has dealt damage to this game.
//! Set: ME1 #69 — Masters Edition | Scryfall ID: 49058733-6fd7-456f-a47b-10ebf7fb5232 | Oracle ID: 19348c1e-b683-4d2c-b4eb-82e538b314b0

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_FALLEN,
    oracle_id = "19348c1e-b683-4d2c-b4eb-82e538b314b0",
    scryfall_id = "49058733-6fd7-456f-a47b-10ebf7fb5232",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Fallen",
        mana_cost = mana!("{1}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::DealDamageToEachOpponentAndPlaneswalkerDamagedThisGame {
                amount: Amount::Fixed(1),
            }
        ]
    )],
);
