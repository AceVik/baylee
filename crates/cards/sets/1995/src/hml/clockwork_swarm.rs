//! Clockwork Swarm — {4} — Artifact Creature — Insect
//! Oracle: This creature enters with four +1/+0 counters on it.
//! Oracle: This creature can't be blocked by Walls.
//! Oracle: At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it.
//! Oracle: {X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than four. Activate only during your upkeep.
//! Set: ME4 #192 — Masters Edition IV | Scryfall ID: a9330b88-af2e-4590-b701-cf4041c3e698 | Oracle ID: a6091c3e-b227-4e84-b75d-69512a0bddfb
// IMPLEMENTED — enters with four +1/+0 counters, can't be blocked by Walls, loses a +1/+0 counter at end of combat after attacking or blocking, and capped upkeep refill.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

card!(
    index = index::CLOCKWORK_SWARM,
    oracle_id = "a6091c3e-b227-4e84-b75d-69512a0bddfb",
    scryfall_id = "a9330b88-af2e-4590-b701-cf4041c3e698",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Clockwork Swarm",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::INSECT],
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
        static_ability!(Filter::This, Modifier::CantBeBlockedBy(&WALL)),
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
