//! Rukh Egg — {3}{R} — Creature — Bird Egg
//! Oracle: When this creature dies, create a 4/4 red Bird creature token with flying at the beginning of the next end step.
//! Set: 9ED #214 — Ninth Edition | Scryfall ID: e4bfc662-edfb-43ff-8022-1db971b1de22 | Oracle ID: 98116aec-2ab1-4bee-b727-9feff6274825
// IMPLEMENTED — the dies trigger leaves a delayed trigger that creates the Bird at the next end step.

use crate::generated_tokens::BIRD_4_4_RED_FLYING as BIRD;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RUKH_EGG,
    oracle_id = "98116aec-2ab1-4bee-b727-9feff6274825",
    scryfall_id = "e4bfc662-edfb-43ff-8022-1db971b1de22",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rukh Egg",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::EGG],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::AtNextEndStep {
            effects: &[Effect::CreateToken { token: &BIRD }]
        }]
    )],
);
