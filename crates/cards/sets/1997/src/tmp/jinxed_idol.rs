//! Jinxed Idol — {2} — Artifact
//! Oracle: At the beginning of your upkeep, this artifact deals 2 damage to you.
//! Oracle: Sacrifice a creature: Target opponent gains control of this artifact.
//! Set: TPR #224 — Tempest Remastered | Scryfall ID: 269da2cd-0a50-4b07-a343-35371a9dcbb4 | Oracle ID: 89b1eab9-4d24-4367-99f7-263349f69c3a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JINXED_IDOL,
    oracle_id = "89b1eab9-4d24-4367-99f7-263349f69c3a",
    scryfall_id = "269da2cd-0a50-4b07-a343-35371a9dcbb4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jinxed Idol",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::You),
                amount: Amount::Fixed(2),
            }],
        ),
        activated!(
            cost!(Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::TargetPlayerOrController,
            }],
            target = Some(TargetSpec::AnyOpponent),
        ),
    ],
);
