//! Fling — {1}{R} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Fling deals damage equal to the sacrificed creature's power to any target.
//! Set: JMP #320 — Jumpstart | Scryfall ID: 8f42d773-c742-4465-b6d5-31feaba49146 | Oracle ID: 24227761-b50e-4b9e-93a2-e82d053b3e3d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLING,
    oracle_id = "24227761-b50e-4b9e-93a2-e82d053b3e3d",
    scryfall_id = "8f42d773-c742-4465-b6d5-31feaba49146",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fling",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::SacrificedPower,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
