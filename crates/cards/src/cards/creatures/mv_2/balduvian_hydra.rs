//! Balduvian Hydra — {X}{R}{R} — Creature — Hydra
//! Oracle: This creature enters with X +1/+0 counters on it.
//! Oracle: Remove a +1/+0 counter from this creature: Prevent the next 1 damage that would be dealt to it this turn.
//! Oracle: {R}{R}{R}: Put a +1/+0 counter on this creature. Activate only during your upkeep.
//! Set: ME2 #118 — Masters Edition II | Scryfall ID: 6c8128f0-7fbf-401e-b185-f85f2794f626 | Oracle ID: 29a12f75-d66d-457f-85c4-6b9450adc0d8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALDUVIAN_HYDRA,
    oracle_id = "29a12f75-d66d-457f-85c4-6b9450adc0d8",
    scryfall_id = "6c8128f0-7fbf-401e-b185-f85f2794f626",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balduvian Hydra",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HYDRA],
        power = Some(0),
        toughness = Some(1),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
            amount: Amount::X,
        }],
    ),],
    abilities = &[
        activated!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0,
                },
                n: 1,
            }),
            &[Effect::PreventNextDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::ThisObject,
            }]
        ),
        activated!(
            cost!("{R}{R}{R}"),
            &[Effect::AddCounter {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0,
                },
                amount: Amount::Fixed(1),
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ]))
        ),
    ],
);
