//! Fireblast — {4}{R}{R} — Instant
//! Oracle: You may sacrifice two Mountains rather than pay this spell's mana cost.
//! Oracle: Fireblast deals 4 damage to any target.
//! Set: DMR #119 — Dominaria Remastered | Scryfall ID: a0ab67cc-b553-4451-9cda-0e5fe3303940 | Oracle ID: 9dd7f27a-e862-47d7-9158-034cf4d353b8
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIREBLAST,
    oracle_id = "9dd7f27a-e862-47d7-9158-034cf4d353b8",
    scryfall_id = "a0ab67cc-b553-4451-9cda-0e5fe3303940",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fireblast",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[
                    CostPart::Sacrifice(&Filter::HasSubtype(subtypes::land::MOUNTAIN)),
                    CostPart::Sacrifice(&Filter::HasSubtype(subtypes::land::MOUNTAIN)),
                ],
            },
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
