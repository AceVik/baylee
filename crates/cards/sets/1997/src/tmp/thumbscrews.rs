//! Thumbscrews — {2} — Artifact
//! Oracle: At the beginning of your upkeep, if you have five or more cards in hand, this artifact deals 1 damage to target opponent or planeswalker.
//! Set: TMP #312 — Tempest | Scryfall ID: 71025a6b-3aed-4943-a907-9584d135f6c0 | Oracle ID: a8f1f55a-7a85-4338-b4d1-3d28f10f69a7

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THUMBSCREWS,
    oracle_id = "a8f1f55a-7a85-4338-b4d1-3d28f10f69a7",
    scryfall_id = "71025a6b-3aed-4943-a907-9584d135f6c0",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thumbscrews",
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
        condition = Some(Condition::AmountAtLeast(
            Amount::HandSize(PlayerRel::You),
            5
        )),
        targets = Some(TargetReq::one(TargetSpec::OpponentOrObject(
            &Filter::PLANESWALKER
        ))),
    )],
);
