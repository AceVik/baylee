//! Desert — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step.
//! Set: AFC #233 — Forgotten Realms Commander | Scryfall ID: c74e13eb-6f82-4db1-9d0d-8310f48d9f6d | Oracle ID: 195107ad-879d-4b02-a44a-a3ba70fedf88
// IMPLEMENTED — {T}: Add {C}; the damage ability is offered only in the end
// of combat step (Condition::DuringStep), when attackers are still attacking
// (CR 511.3).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESERT,
    oracle_id = "195107ad-879d-4b02-a44a-a3ba70fedf88",
    scryfall_id = "c74e13eb-6f82-4db1-9d0d-8310f48d9f6d",
    faces = &[face!(
        name = "Desert",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
            }],
            target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE)),
            condition = Some(Condition::DuringStep(StepKind::CombatEnd)),
        ),
    ],
);
