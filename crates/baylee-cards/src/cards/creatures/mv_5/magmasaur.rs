//! Magmasaur — {3}{R}{R} — Creature — Elemental Dinosaur
//! Oracle: This creature enters with five +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may remove a +1/+1 counter from this creature. If you don't, sacrifice this creature and it deals damage equal to the number of +1/+1 counters on it to each creature without flying and each player.
//! Set: TPR #141 — Tempest Remastered | Scryfall ID: dc628d2f-ca19-4ef0-8b8d-d3a831579b52 | Oracle ID: da38bfdf-b4c8-4ffe-ad53-b42c317cae5f
// PARTIAL: enters with five +1/+1 counters and lets you remove a +1/+1 counter at the beginning of your upkeep; sacrificing itself and dealing damage equal to its +1/+1 counters to each creature without flying and each player if you don't is not in Effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGMASAUR,
    oracle_id = "da38bfdf-b4c8-4ffe-ad53-b42c317cae5f",
    scryfall_id = "dc628d2f-ca19-4ef0-8b8d-d3a831579b52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "enters with five +1/+1 counters and lets you remove a +1/+1 counter at the beginning of your upkeep; sacrificing itself and dealing damage equal to its +1/+1 counters to each creature without flying and each player if you don't is not in Effect",
    ),
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
        &[Effect::MayDo {
            effects: &[Effect::RemoveCounterSelf {
                kind: CounterKind::P1P1,
                n: 1,
            }],
        }],
    )],
);
