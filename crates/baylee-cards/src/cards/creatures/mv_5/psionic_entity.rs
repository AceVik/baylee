//! Psionic Entity — {4}{U} — Creature — Illusion
//! Oracle: {T}: This creature deals 2 damage to any target and 3 damage to itself.
//! Set: 4ED #95 — Fourth Edition | Scryfall ID: 6641a587-c066-4f0a-a951-b91d3f749eb2 | Oracle ID: 37aae6d1-1de0-47d1-85dd-d4a6d9635c05
// IMPLEMENTED — {T}: 2 damage to any target, then 3 damage to this creature
// (`DealDamageEach` over `Filter::This` names the resolving source without
// making it a target, the way `PumpFilter` does).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PSIONIC_ENTITY,
    oracle_id = "37aae6d1-1de0-47d1-85dd-d4a6d9635c05",
    scryfall_id = "6641a587-c066-4f0a-a951-b91d3f749eb2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Psionic Entity",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            },
            Effect::DealDamageEach {
                amount: Amount::Fixed(3),
                filter: &Filter::This,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )],
);
