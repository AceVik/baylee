//! Ebon Praetor — {4}{B}{B} — Creature — Avatar Praetor
//! Oracle: First strike, trample
//! Oracle: At the beginning of your upkeep, put a -2/-2 counter on this creature.
//! Oracle: Sacrifice a creature: Remove a -2/-2 counter from this creature. If the sacrificed creature was a Thrull, put a +1/+0 counter on this creature. Activate only during your upkeep and only once each turn.
//! Set: ME2 #89 — Masters Edition II | Scryfall ID: 6438dc8e-eb21-4db7-aeab-bb062acd6029 | Oracle ID: 0501af1f-c67c-4cb1-aced-4be426c83316
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const THRULL: Filter = Filter::HasSubtype(subtypes::creature::THRULL);

card!(
    index = index::EBON_PRAETOR,
    oracle_id = "0501af1f-c67c-4cb1-aced-4be426c83316",
    scryfall_id = "6438dc8e-eb21-4db7-aeab-bb062acd6029",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FIRST_STRIKE.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ebon Praetor",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::PRAETOR],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounterSource {
                kind: CounterKind::Minus {
                    power: 2,
                    toughness: 2,
                },
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(Sacrifice(&Filter::CREATURE)),
            &[
                Effect::RemoveCounterSelf {
                    kind: CounterKind::Minus {
                        power: 2,
                        toughness: 2,
                    },
                    n: 1,
                },
                Effect::IfCondition {
                    condition: Condition::SacrificedForCost(&THRULL),
                    then: &[Effect::AddCounterSource {
                        kind: CounterKind::Plus {
                            power: 1,
                            toughness: 0,
                        },
                        amount: Amount::Fixed(1),
                    }],
                    otherwise: &[],
                },
            ],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
            limit = ActivationLimit::PerTurn(1),
        ),
    ],
);
