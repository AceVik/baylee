//! Faith Healer — {1}{W} — Creature — Human Cleric
//! Oracle: Sacrifice an enchantment: You gain life equal to the sacrificed enchantment's mana value.
//! Set: USG #14 — Urza's Saga | Scryfall ID: 5d51ff6a-ba1b-4015-8d0b-df1a1bb5a0c1 | Oracle ID: c657e110-c204-48df-a396-9822ac48798d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FAITH_HEALER,
    oracle_id = "c657e110-c204-48df-a396-9822ac48798d",
    scryfall_id = "5d51ff6a-ba1b-4015-8d0b-df1a1bb5a0c1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Faith Healer",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::ENCHANTMENT)),
        &[Effect::GainLife {
            amount: Amount::SacrificedManaValue,
        }],
    )],
);
