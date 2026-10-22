//! Searing Touch — {R} — Instant
//! Oracle: Buyback {4} (You may pay an additional {4} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Searing Touch deals 1 damage to any target.
//! Set: TPR #155 — Tempest Remastered | Scryfall ID: 34406508-da49-4b63-b123-55f0924a0675 | Oracle ID: 1cd9a3ba-a115-4e59-983d-605e4435f3a8
// PARTIAL: deals 1 damage to any target; buyback {4} is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEARING_TOUCH,
    oracle_id = "1cd9a3ba-a115-4e59-983d-605e4435f3a8",
    scryfall_id = "34406508-da49-4b63-b123-55f0924a0675",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("deals 1 damage to any target; buyback {4} is not supported"),
    faces = &[face!(
        name = "Searing Touch",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
