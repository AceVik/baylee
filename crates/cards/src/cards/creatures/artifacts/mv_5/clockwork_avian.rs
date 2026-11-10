//! Clockwork Avian — {5} — Artifact Creature — Bird
//! Oracle: Flying
//! Oracle: This creature enters with four +1/+0 counters on it.
//! Oracle: At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it.
//! Oracle: {X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than four. Activate only during your upkeep.
//! Set: ME4 #190 — Masters Edition IV | Scryfall ID: 8013397b-55c4-4439-a320-9c9feb6864c2 | Oracle ID: 3d5b71d4-ed5e-4c6d-be70-bebbb1475257
// IMPLEMENTED — flying, four +1/+0 counters on entry, one lost after attacking or blocking, and the capped upkeep refill.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLOCKWORK_AVIAN,
    oracle_id = "3d5b71d4-ed5e-4c6d-be70-bebbb1475257",
    scryfall_id = "8013397b-55c4-4439-a320-9c9feb6864c2",
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Clockwork Avian",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::BIRD],
        power = Some(0),
        toughness = Some(4),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::Plus {
                power: 1,
                toughness: 0
            },
            amount: Amount::Fixed(4)
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatEnd,
                whose: PlayerRel::EachPlayer
            },
            &[Effect::RemoveCounterSelf {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0
                },
                n: 1,
            }],
            condition = Some(Condition::AttackedOrBlockedThisCombat),
        ),
        activated!(
            cost!("{X}", TapSelf),
            &[Effect::AddCountersUpTo {
                kind: CounterKind::Plus {
                    power: 1,
                    toughness: 0
                },
                amount: Amount::X,
                maximum: 4,
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
