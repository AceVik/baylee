//! Jinxed Choker — {3} — Artifact
//! Oracle: At the beginning of your end step, target opponent gains control of this artifact and puts a charge counter on it.
//! Oracle: At the beginning of your upkeep, this artifact deals damage to you equal to the number of charge counters on it.
//! Oracle: {3}: Put a charge counter on this artifact or remove one from it.
//! Set: MRD #189 — Mirrodin | Scryfall ID: 987910b0-0419-45ff-bda6-c6683fd00e49 | Oracle ID: 3241abc1-d739-4ac3-93d1-7fc2898b943b
// IMPLEMENTED — transcoded from the card's rules, adopted from `xtask codegen`.
// Adopted because the reference script prints "put … or remove one" as two
// `{3}` abilities, which the reader transcribes line by line. The ruling
// (2004-12-01) makes it one ability whose add-or-remove is chosen on
// resolution, so the two lines are one `Effect::ChooseOne` here.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JINXED_CHOKER,
    oracle_id = "3241abc1-d739-4ac3-93d1-7fc2898b943b",
    scryfall_id = "987910b0-0419-45ff-bda6-c6683fd00e49",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jinxed Choker",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You
            },
            &[
                Effect::ChangeController {
                    new_controller: PlayerRel::Chosen
                },
                Effect::AddCounter {
                    kind: CounterKind::Charge,
                    amount: Amount::Fixed(1)
                }
            ],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(CounterKind::Charge),
                target: TargetSpec::Player(PlayerRel::You)
            }]
        ),
        activated!(
            cost!("{3}"),
            &[Effect::ChooseOne {
                choices: &[
                    Effect::AddCounter {
                        kind: CounterKind::Charge,
                        amount: Amount::Fixed(1)
                    },
                    Effect::RemoveCounterSelf {
                        kind: CounterKind::Charge,
                        n: 1
                    },
                ]
            }]
        ),
    ],
);
