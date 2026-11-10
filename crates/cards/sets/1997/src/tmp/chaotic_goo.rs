//! Chaotic Goo — {2}{R}{R} — Creature — Ooze
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may flip a coin. If you win the flip, put a +1/+1 counter on this creature. If you lose the flip, remove a +1/+1 counter from this creature.
//! Set: TMP #168 — Tempest | Scryfall ID: 0e9881a4-3078-4fe0-be09-54ddad1d18a0 | Oracle ID: d1005625-0a78-4f8e-bf51-80458fc0e9d6
// IMPLEMENTED — Enters with three +1/+1 counters and optionally flips a coin at your upkeep to add or remove a +1/+1 counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAOTIC_GOO,
    oracle_id = "d1005625-0a78-4f8e-bf51-80458fc0e9d6",
    scryfall_id = "0e9881a4-3078-4fe0-be09-54ddad1d18a0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chaotic Goo",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OOZE],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::FlipCoin {
                won: &[Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                }],
                lost: &[Effect::RemoveCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 1,
                }],
            }],
        }]
    )],
);
