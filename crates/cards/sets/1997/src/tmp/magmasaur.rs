//! Magmasaur — {3}{R}{R} — Creature — Elemental Dinosaur
//! Oracle: This creature enters with five +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may remove a +1/+1 counter from this creature. If you don't, sacrifice this creature and it deals damage equal to the number of +1/+1 counters on it to each creature without flying and each player.
//! Set: TPR #141 — Tempest Remastered | Scryfall ID: dc628d2f-ca19-4ef0-8b8d-d3a831579b52 | Oracle ID: da38bfdf-b4c8-4ffe-ad53-b42c317cae5f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const EXPLODE: &[Effect] = &[
    Effect::SacrificeSelf,
    Effect::DealDamageEach {
        amount: Amount::CountersOnSource(CounterKind::P1P1),
        filter: &Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
        ]),
    },
    Effect::DealDamage {
        amount: Amount::CountersOnSource(CounterKind::P1P1),
        target: TargetSpec::Player(PlayerRel::EachPlayer),
    },
];

card!(
    index = index::MAGMASAUR,
    oracle_id = "da38bfdf-b4c8-4ffe-ad53-b42c317cae5f",
    scryfall_id = "dc628d2f-ca19-4ef0-8b8d-d3a831579b52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Magmasaur",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::DINOSAUR],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(5),
        }],
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::IfCondition {
            condition: Condition::CountersOnSelf(CounterKind::P1P1, 1),
            then: &[Effect::ChooseOne {
                choices: &[
                    Effect::RemoveCounterSelf {
                        kind: CounterKind::P1P1,
                        n: 1
                    },
                    Effect::Sequence(EXPLODE),
                ],
            }],
            otherwise: EXPLODE,
        }],
    )],
);
