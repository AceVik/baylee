//! Rock Hydra — {X}{R}{R} — Creature — Hydra
//! Oracle: This creature enters with X +1/+1 counters on it.
//! Oracle: For each 1 damage that would be dealt to this creature, if it has a +1/+1 counter on it, remove a +1/+1 counter from it and prevent that 1 damage.
//! Oracle: {R}: Prevent the next 1 damage that would be dealt to this creature this turn.
//! Oracle: {R}{R}{R}: Put a +1/+1 counter on this creature. Activate only during your upkeep.
//! Set: ME4 #133 — Masters Edition IV | Scryfall ID: 28ab1823-e11e-48ae-8406-e025af32f407 | Oracle ID: aff84707-f5f8-4f53-869e-feec78da8d8d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROCK_HYDRA,
    oracle_id = "aff84707-f5f8-4f53-869e-feec78da8d8d",
    scryfall_id = "28ab1823-e11e-48ae-8406-e025af32f407",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rock Hydra",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HYDRA],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::X,
        }],
    ),],
    // The {R} shield keeps index 0, which it had before the other two were
    // written: a seat's standing answers are keyed by the index.
    abilities = &[
        activated!(
            cost!("{R}"),
            &[Effect::PreventNextDamage {
                target: TargetSpec::ThisObject,
                amount: Amount::Fixed(1)
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::CountersPreventDamage(CounterKind::P1P1)
        ),
        activated!(
            cost!("{R}{R}{R}"),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
