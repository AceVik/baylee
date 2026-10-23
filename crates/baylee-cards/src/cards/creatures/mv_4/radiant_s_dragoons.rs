//! Radiant's Dragoons — {3}{W} — Creature — Human Soldier
//! Oracle: Echo {3}{W} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, you gain 5 life.
//! Set: ULG #21 — Urza's Legacy | Scryfall ID: 8a0f39de-6ad2-410c-bc6c-75fd3c8d159b | Oracle ID: a0fe91a8-633c-4902-bc75-da0a5aa76992
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RADIANT_S_DRAGOONS,
    oracle_id = "a0fe91a8-633c-4902-bc75-da0a5aa76992",
    scryfall_id = "8a0f39de-6ad2-410c-bc6c-75fd3c8d159b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Radiant's Dragoons",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(5),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{3}{W}"),
        },
        triggered!(Trigger::ETB, &[Effect::gain_life(5)]),
    ],
);
