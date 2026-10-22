//! Torture Chamber — {3} — Artifact
//! Oracle: At the beginning of your upkeep, put a pain counter on this artifact.
//! Oracle: At the beginning of your end step, this artifact deals damage to you equal to the number of pain counters on it.
//! Oracle: {1}, {T}, Remove all pain counters from this artifact: It deals damage to target creature equal to the number of pain counters removed this way.
//! Set: TMP #313 — Tempest | Scryfall ID: 5648158d-38d4-4167-8af5-ee5d7d6fd7cb | Oracle ID: 12436f2f-eaed-4b46-8570-81316ce15962
// PARTIAL: puts a pain counter on itself on your upkeep, deals damage equal to the number of pain counters on it to you on your end step, and activates {1}, {T} to deal damage equal to its pain counters to target creature and remove all pain counters from itself on resolution; removing all pain counters as an activation cost is not on CostPart.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORTURE_CHAMBER,
    oracle_id = "12436f2f-eaed-4b46-8570-81316ce15962",
    scryfall_id = "5648158d-38d4-4167-8af5-ee5d7d6fd7cb",
    coverage = Coverage::Partial(
        "puts a pain counter on itself on your upkeep, deals damage equal to the number of pain counters on it to you on your end step, and activates {1}, {T} to deal damage equal to its pain counters to target creature and remove all pain counters from itself on resolution; removing all pain counters as an activation cost is not on CostPart",
    ),
    faces = &[face!(
        name = "Torture Chamber",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounterSource {
                kind: counters::PAIN,
                amount: Amount::Fixed(1),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::PAIN),
                target: TargetSpec::Player(PlayerRel::You),
            }],
        ),
        activated!(
            cost!("{1}", TapSelf),
            &[
                Effect::DealDamage {
                    amount: Amount::CountersOnSource(counters::PAIN),
                    target: TargetSpec::Object(&Filter::CREATURE),
                },
                Effect::RemoveAllCountersSelf {
                    kind: counters::PAIN,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
