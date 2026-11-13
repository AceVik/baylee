//! Warrior Angel — {4}{W}{W} — Creature — Angel Warrior
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Set: STH #24 — Stronghold | Scryfall ID: 81d2c6e3-e556-4cc6-94b6-fdbc62d6853e | Oracle ID: 97140551-341a-40a1-93fe-0fcaa0c3be33
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARRIOR_ANGEL,
    oracle_id = "97140551-341a-40a1-93fe-0fcaa0c3be33",
    scryfall_id = "81d2c6e3-e556-4cc6-94b6-fdbc62d6853e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Warrior Angel",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamage(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }],
    ),],
);
