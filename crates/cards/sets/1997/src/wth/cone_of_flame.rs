//! Cone of Flame — {3}{R}{R} — Sorcery
//! Oracle: Cone of Flame deals 1 damage to any target, 2 damage to another target, and 3 damage to a third target.
//! Set: W16 #12 — Welcome Deck 2016 | Scryfall ID: 9b434a60-0875-47f4-a626-2c9bfa3c9b3b | Oracle ID: 62879f3d-ed6d-4004-8196-f3fd7f614695
// IMPLEMENTED — deals 1, 2, and 3 damage to three distinct targets via WithSecondTarget and WithThirdTarget.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONE_OF_FLAME,
    oracle_id = "62879f3d-ed6d-4004-8196-f3fd7f614695",
    scryfall_id = "9b434a60-0875-47f4-a626-2c9bfa3c9b3b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cone of Flame",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            },
            Effect::WithSecondTarget(&[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }]),
            Effect::WithThirdTarget(&[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            }]),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        second_targets = Some(
            TargetReq::one(TargetSpec::AnyTarget)
                .with_constraint(TargetConstraint::DistinctFromPriorTargetGroups),
        ),
        third_targets = Some(
            TargetReq::one(TargetSpec::AnyTarget)
                .with_constraint(TargetConstraint::DistinctFromPriorTargetGroups),
        ),
    )],
);
