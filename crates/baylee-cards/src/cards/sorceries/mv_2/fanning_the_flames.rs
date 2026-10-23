//! Fanning the Flames — {X}{R}{R} — Sorcery
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Fanning the Flames deals X damage to any target.
//! Set: TPR #130 — Tempest Remastered | Scryfall ID: 198b8ff9-f49a-45e2-bd33-c8b805f7ac42 | Oracle ID: 1d5f0a1c-88e0-4603-97af-8ed47766812e
// PARTIAL: deals X damage to any target; buyback {3} is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FANNING_THE_FLAMES,
    oracle_id = "1d5f0a1c-88e0-4603-97af-8ed47766812e",
    scryfall_id = "198b8ff9-f49a-45e2-bd33-c8b805f7ac42",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("deals X damage to any target; buyback {3} is not supported"),
    faces = &[face!(
        name = "Fanning the Flames",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::X,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
