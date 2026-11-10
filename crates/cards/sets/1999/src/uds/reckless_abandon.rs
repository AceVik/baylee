//! Reckless Abandon — {R} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Reckless Abandon deals 4 damage to any target.
//! Set: DDN #20 — Duel Decks: Speed vs. Cunning | Scryfall ID: 4327fd85-eb89-4707-80f8-bdf41179eda5 | Oracle ID: f24d5187-0f80-4528-bf44-03ae146f3551

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECKLESS_ABANDON,
    oracle_id = "f24d5187-0f80-4528-bf44-03ae146f3551",
    scryfall_id = "4327fd85-eb89-4707-80f8-bdf41179eda5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reckless Abandon",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
