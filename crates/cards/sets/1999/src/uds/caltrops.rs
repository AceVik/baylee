//! Caltrops — {3} — Artifact
//! Oracle: Whenever a creature attacks, this artifact deals 1 damage to it.
//! Set: 7ED #288 — Seventh Edition | Scryfall ID: 8769b55a-a0a1-4b6f-8c80-669385a34425 | Oracle ID: b13f97b3-56ae-4131-a93b-b162c8ff7cca
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CALTROPS,
    oracle_id = "b13f97b3-56ae-4131-a93b-b162c8ff7cca",
    scryfall_id = "8769b55a-a0a1-4b6f-8c80-669385a34425",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Caltrops",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::CREATURE),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::EventObject,
        }],
    )],
);
