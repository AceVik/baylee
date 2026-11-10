//! Reckless Embermage — {3}{R} — Creature — Human Wizard
//! Oracle: {1}{R}: This creature deals 1 damage to any target and 1 damage to itself.
//! Set: 7ED #212 — Seventh Edition | Scryfall ID: b0f75ed4-b96c-444b-ac91-8aaa02f32f2d | Oracle ID: 5771296a-16f8-423e-9b83-8e6f687cbb34
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RECKLESS_EMBERMAGE,
    oracle_id = "5771296a-16f8-423e-9b83-8e6f687cbb34",
    scryfall_id = "b0f75ed4-b96c-444b-ac91-8aaa02f32f2d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reckless Embermage",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}"),
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            },
            Effect::DealDamageEach {
                amount: Amount::Fixed(1),
                filter: &Filter::This,
            },
        ],
        target = Some(TargetSpec::AnyTarget),
    )],
);
