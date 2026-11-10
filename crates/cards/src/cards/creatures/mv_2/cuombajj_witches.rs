//! Cuombajj Witches — {B}{B} — Creature — Human Wizard
//! Oracle: {T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice.
//! Set: CMR #116 — Commander Legends | Scryfall ID: 6a26e910-275a-4981-831b-bfed936a7e3f | Oracle ID: 638eeb16-9e0e-4cc6-b97e-8ff0df81ca58
// Full paired target damage with opponent-chosen second target.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CUOMBAJJ_WITCHES,
    oracle_id = "638eeb16-9e0e-4cc6-b97e-8ff0df81ca58",
    scryfall_id = "6a26e910-275a-4981-831b-bfed936a7e3f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cuombajj Witches",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::DealDamageToBothTargetInstances {
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::AnyTarget),
        second_targets = Some(TargetReq::one(TargetSpec::AnyTarget).chosen_by_opponent()),
    )],
);
