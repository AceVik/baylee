//! Mogg Bombers — {3}{R} — Creature — Goblin
//! Oracle: When another creature enters, sacrifice this creature and it deals 3 damage to target player or planeswalker.
//! Set: STH #91 — Stronghold | Scryfall ID: 16064ad4-eec2-4c32-b66d-a2bdb1a6191f | Oracle ID: 69812cd2-9885-45ed-9d79-658597bfeb8b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Another]);

card!(
    index = index::MOGG_BOMBERS,
    oracle_id = "69812cd2-9885-45ed-9d79-658597bfeb8b",
    scryfall_id = "16064ad4-eec2-4c32-b66d-a2bdb1a6191f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mogg Bombers",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&ANOTHER_CREATURE),
        &[
            Effect::SacrificeSelf,
            Effect::DealDamage {
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
                amount: Amount::Fixed(3),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER
        ))),
    )],
);
