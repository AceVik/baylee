//! Trade Caravan — {W} — Creature — Human Nomad
//! Oracle: At the beginning of your upkeep, put a currency counter on this creature.
//! Oracle: Remove two currency counters from this creature: Untap target basic land. Activate only during an opponent's upkeep.
//! Set: HML #19a — Homelands | Scryfall ID: fdcc821b-8125-4585-ba44-4bf8c1873fbf | Oracle ID: 8faccd56-dfc2-4cfc-880c-db40b935336a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRADE_CARAVAN,
    oracle_id = "8faccd56-dfc2-4cfc-880c-db40b935336a",
    scryfall_id = "fdcc821b-8125-4585-ba44-4bf8c1873fbf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Trade Caravan",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::CURRENCY,
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(RemoveCounterSelf {
                kind: counters::CURRENCY,
                n: 2,
            }),
            &[Effect::UntapTarget],
            target = Some(TargetSpec::Object(&Filter::BASIC_LAND)),
            condition = Some(Condition::All(&[
                Condition::OpponentsTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
