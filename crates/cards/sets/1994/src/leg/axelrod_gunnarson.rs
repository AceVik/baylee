//! Axelrod Gunnarson — {4}{B}{B}{R}{R} — Legendary Creature — Giant
//! Oracle: Trample
//! Oracle: Whenever a creature dealt damage by Axelrod Gunnarson this turn dies, you gain 1 life and Axelrod Gunnarson deals 1 damage to target player or planeswalker.
//! Set: ME3 #143 — Masters Edition III | Scryfall ID: 97f2b387-661e-435f-96c9-d3d5a601caa9 | Oracle ID: c640350b-16a5-4227-87fa-8b24fdb367c7

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AXELROD_GUNNARSON,
    oracle_id = "c640350b-16a5-4227-87fa-8b24fdb367c7",
    scryfall_id = "97f2b387-661e-435f-96c9-d3d5a601caa9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Axelrod Gunnarson",
        mana_cost = mana!("{4}{B}{B}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
        &[
            Effect::gain_life(1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)
            }
        ],
        targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER
        )))
    )],
);
