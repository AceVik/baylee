//! Elite Javelineer — {2}{W} — Creature — Human Soldier
//! Oracle: Whenever this creature blocks, it deals 1 damage to target attacking creature.
//! Set: 8ED #19 — Eighth Edition | Scryfall ID: ba1839ea-567e-44eb-983f-c1a570ba5c82 | Oracle ID: 649300e2-c491-435d-92d5-f1b88f0e2e2e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELITE_JAVELINEER,
    oracle_id = "649300e2-c491-435d-92d5-f1b88f0e2e2e",
    scryfall_id = "ba1839ea-567e-44eb-983f-c1a570ba5c82",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elite Javelineer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        ))),
    )],
);
