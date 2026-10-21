//! Labyrinth Minotaur — {3}{U} — Creature — Minotaur
//! Oracle: Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step.
//! Set: ME3 #39 — Masters Edition III | Scryfall ID: e5a0658a-dcf6-498f-bac2-da8b3ce5022e | Oracle ID: 1e7b075b-1c0f-4c44-a597-5c167a786cb1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LABYRINTH_MINOTAUR,
    oracle_id = "1e7b075b-1c0f-4c44-a597-5c167a786cb1",
    scryfall_id = "e5a0658a-dcf6-498f-bac2-da8b3ce5022e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Labyrinth Minotaur",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINOTAUR],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&Filter::CREATURE),
        &[Effect::SuppressUntap {
            target: TargetSpec::EventObject,
            steps: 1,
        }],
    )],
);
