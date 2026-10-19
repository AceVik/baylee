//! Goblin Grenade — {R} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a Goblin.
//! Oracle: Goblin Grenade deals 5 damage to any target.
//! Set: DDT #45 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: b0e0ef27-3db2-4976-b9db-13e3d7cd795d | Oracle ID: 20aecb7b-7e37-4efe-ac27-f9f9042203f4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_GOBLIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::GOBLIN_GRENADE,
    oracle_id = "20aecb7b-7e37-4efe-ac27-f9f9042203f4",
    scryfall_id = "b0e0ef27-3db2-4976-b9db-13e3d7cd795d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Grenade",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&YOUR_GOBLIN)],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(5),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    ),],
);
