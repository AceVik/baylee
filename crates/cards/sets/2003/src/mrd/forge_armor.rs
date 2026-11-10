//! Forge Armor — {4}{R} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice an artifact.
//! Oracle: Put X +1/+1 counters on target creature, where X is the sacrificed artifact's mana value.
//! Set: MRD #92 — Mirrodin | Scryfall ID: 2873b6d5-af76-498c-bc2d-a26c36be3cbd | Oracle ID: 7d4d9bfa-7e85-461b-bd62-1de97690a110
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGE_ARMOR,
    oracle_id = "7d4d9bfa-7e85-461b-bd62-1de97690a110",
    scryfall_id = "2873b6d5-af76-498c-bc2d-a26c36be3cbd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Forge Armor",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_ARTIFACT)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::SacrificedManaValue,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
