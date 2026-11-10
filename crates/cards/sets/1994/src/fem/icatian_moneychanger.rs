//! Icatian Moneychanger — {W} — Creature — Human
//! Oracle: This creature enters with three credit counters on it.
//! Oracle: When this creature enters, it deals 3 damage to you.
//! Oracle: At the beginning of your upkeep, put a credit counter on this creature.
//! Oracle: Sacrifice this creature: You gain 1 life for each credit counter on this creature. Activate only during your upkeep.
//! Set: FEM #10a — Fallen Empires | Scryfall ID: b3d502d4-4a96-47b3-ae26-8b2c9f36623d | Oracle ID: 9b81aa32-e0f7-493f-92b8-85c2c46aef91
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICATIAN_MONEYCHANGER,
    oracle_id = "9b81aa32-e0f7-493f-92b8-85c2c46aef91",
    scryfall_id = "b3d502d4-4a96-47b3-ae26-8b2c9f36623d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Icatian Moneychanger",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(2),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::CREDIT,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::You),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::CREDIT,
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::GainLife {
                amount: Amount::CountersOnSource(counters::CREDIT),
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
