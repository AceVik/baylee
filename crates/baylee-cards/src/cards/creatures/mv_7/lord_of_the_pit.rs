//! Lord of the Pit — {4}{B}{B}{B} — Creature — Demon
//! Oracle: Flying, trample
//! Oracle: At the beginning of your upkeep, sacrifice a creature other than this creature. If you can't, this creature deals 7 damage to you.
//! Set: IMA #96 — Iconic Masters | Scryfall ID: 5b626ab2-4cf6-42e8-a7e2-9c9f310d2f00 | Oracle ID: ea152809-85a2-4fde-8251-3b1f267e4443
// IMPLEMENTED — "if you can't" is asked before the sacrifice, over the
// permanents the sacrifice would offer (`Condition::CanSacrifice`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LORD_OF_THE_PIT,
    oracle_id = "ea152809-85a2-4fde-8251-3b1f267e4443",
    scryfall_id = "5b626ab2-4cf6-42e8-a7e2-9c9f310d2f00",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lord of the Pit",
        mana_cost = mana!("{4}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(7),
        toughness = Some(7),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::IfCondition {
            condition: Condition::CanSacrifice(&Filter::ANOTHER_CREATURE),
            then: &[Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::ANOTHER_CREATURE
            }],
            otherwise: &[Effect::DealDamage {
                amount: Amount::Fixed(7),
                target: TargetSpec::Player(PlayerRel::You)
            }],
        }]
    )],
);
