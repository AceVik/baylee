//! Fog Bank — {1}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Flying
//! Oracle: Prevent all combat damage that would be dealt to and dealt by this creature.
//! Set: USG #75 — Urza's Saga | Scryfall ID: 6ade0d30-5a57-439e-95e8-5f865880031f | Oracle ID: 3a2bb47d-228d-499e-a34b-9e10d99e9b2c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOG_BANK,
    oracle_id = "3a2bb47d-228d-499e-a34b-9e10d99e9b2c",
    scryfall_id = "6ade0d30-5a57-439e-95e8-5f865880031f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fog Bank",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::PreventDamageToIt),
        static_ability!(Filter::This, Modifier::PreventDamageFromIt),
    ],
);
