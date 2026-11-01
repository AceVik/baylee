//! Clockwork Steed — {4} — Artifact Creature — Horse
//! Oracle: This creature enters with four +1/+0 counters on it.
//! Oracle: This creature can't be blocked by artifact creatures.
//! Oracle: At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it.
//! Oracle: {X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than four. Activate only during your upkeep.
//! Set: ME2 #205 — Masters Edition II | Scryfall ID: 6d76dab5-a4cb-4799-b7a9-9eb93a5b0eba | Oracle ID: 1d6a1dab-8724-4b07-b109-c6ba2a7ea1ad
// IMPLEMENTED — enters with four +1/+0 counters, can't be blocked by artifact creatures, loses a +1/+0 counter at end of combat after attacking or blocking, and capped upkeep refill.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::CLOCKWORK_STEED,
    oracle_id = "1d6a1dab-8724-4b07-b109-c6ba2a7ea1ad",
    scryfall_id = "6d76dab5-a4cb-4799-b7a9-9eb93a5b0eba",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Clockwork Steed",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::HORSE],
        power = Some(0),
        toughness = Some(3),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
            amount: Amount::Fixed(4),
        }],
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeBlockedBy(&ARTIFACT_CREATURE)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatEnd,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::RemoveCounterSelf {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0,
                },
                n: 1,
            }],
            condition = Some(Condition::AttackedOrBlockedThisCombat)
        ),
        activated!(
            cost!("{X}", TapSelf),
            &[Effect::AddCountersUpTo {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0,
                },
                amount: Amount::X,
                maximum: 4,
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ]))
        ),
    ],
);
