//! Fasting — {W} — Enchantment
//! Oracle: At the beginning of your upkeep, put a hunger counter on this enchantment. Then destroy this enchantment if it has five or more hunger counters on it.
//! Oracle: If you would begin your draw step, you may skip that step instead. If you do, you gain 2 life.
//! Oracle: When you draw a card, destroy this enchantment.
//! Set: DRK #7 — The Dark | Scryfall ID: 8da35f9f-e72c-4154-a212-7de98f84ad7d | Oracle ID: 0d787c6b-ab82-42c5-b840-8f3a09af30ac

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FASTING,
    oracle_id = "0d787c6b-ab82-42c5-b840-8f3a09af30ac",
    scryfall_id = "8da35f9f-e72c-4154-a212-7de98f84ad7d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fasting",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::HUNGER,
                    amount: Amount::Fixed(1),
                },
                Effect::IfCondition {
                    condition: Condition::CountersOnSelf(counters::HUNGER, 5),
                    then: &[Effect::Destroy {
                        target: TargetSpec::ThisObject,
                        no_regen: false,
                    }],
                    otherwise: &[],
                },
            ]
        ),
        AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepGainLife(2)),
        triggered!(
            Trigger::Draws(PlayerRel::You),
            &[Effect::Destroy {
                target: TargetSpec::ThisObject,
                no_regen: false,
            }]
        ),
    ],
);
