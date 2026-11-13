//! Scalding Tongs — {2} — Artifact
//! Oracle: At the beginning of your upkeep, if you have three or fewer cards in hand, this artifact deals 1 damage to target opponent or planeswalker.
//! Set: TMP #307 — Tempest | Scryfall ID: 34136f2c-3edd-4ca3-b1ef-1fdcaa4518a0 | Oracle ID: 939a0cdf-b5e8-4d7d-adb3-f7b2fd7aab8f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCALDING_TONGS,
    oracle_id = "939a0cdf-b5e8-4d7d-adb3-f7b2fd7aab8f",
    scryfall_id = "34136f2c-3edd-4ca3-b1ef-1fdcaa4518a0",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scalding Tongs",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::OpponentOrObject(&Filter::PLANESWALKER),
        }],
        condition = Some(Condition::HandSizeAtMost(3)),
        targets = Some(TargetReq::one(TargetSpec::OpponentOrObject(
            &Filter::PLANESWALKER,
        ))),
    )],
);
