//! Ventifact Bottle — {3} — Artifact
//! Oracle: {X}{1}, {T}: Put X charge counters on this artifact. Activate only as a sorcery.
//! Oracle: At the beginning of your first main phase, if this artifact has a charge counter on it, tap it and remove all charge counters from it. Add {C} for each charge counter removed this way.
//! Set: MIR #323 — Mirage | Scryfall ID: 520f4a24-fb1a-4964-887c-2f08a752fae2 | Oracle ID: 8f167e2f-7b89-4ae2-8332-7c978f71522e
// IMPLEMENTED — {X}{1}, {T} sorcery-speed ability putting X charge counters on it, and first-main-phase trigger tapping it, removing all charge counters, and adding {C} per charge counter removed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VENTIFACT_BOTTLE,
    oracle_id = "8f167e2f-7b89-4ae2-8332-7c978f71522e",
    scryfall_id = "520f4a24-fb1a-4964-887c-2f08a752fae2",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ventifact Bottle",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{X}{1}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::X,
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::FirstMain,
                whose: PlayerRel::You,
            },
            &[
                Effect::TapAll {
                    filter: &Filter::This,
                },
                Effect::mana_dynamic(
                    ManaColor::Colorless,
                    Amount::CountersOnSource(CounterKind::Charge),
                ),
                Effect::RemoveAllCountersSelf {
                    kind: CounterKind::Charge,
                },
            ],
            condition = Some(Condition::CountersOnSelf(CounterKind::Charge, 1)),
        ),
    ],
);
