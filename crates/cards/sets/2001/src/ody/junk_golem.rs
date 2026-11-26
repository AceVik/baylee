//! Junk Golem — {4} — Artifact Creature — Golem
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you remove a +1/+1 counter from it.
//! Oracle: {1}, Discard a card: Put a +1/+1 counter on this creature.
//! Set: ODY #300 — Odyssey | Scryfall ID: c9410da0-5693-456f-afa8-cb92ac176847 | Oracle ID: 044be66d-9334-4d4b-b420-b97a18698e95
// IMPLEMENTED — enters with three +1/+1 counters, upkeep counter-removal tax, and discard pump.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JUNK_GOLEM,
    oracle_id = "044be66d-9334-4d4b-b420-b97a18698e95",
    scryfall_id = "c9410da0-5693-456f-afa8-cb92ac176847",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Junk Golem",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::MayDo {
                    effects: &[Effect::RemoveCounterSelf {
                        kind: CounterKind::P1P1,
                        n: 1,
                    }],
                },
                Effect::IfCondition {
                    condition: Condition::AmountAtLeast(
                        Amount::ThisWay(ThisWay::CountersRemoved(CounterKind::P1P1)),
                        1,
                    ),
                    then: &[],
                    otherwise: &[Effect::SacrificeSelf],
                },
            ],
        ),
        activated!(
            cost!("{1}", Discard(&Filter::Any)),
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
