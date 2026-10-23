//! Fledgling Osprey — {U} — Creature — Bird
//! Oracle: This creature has flying as long as it's enchanted.
//! Set: UDS #33 — Urza's Destiny | Scryfall ID: 8cd46bfa-ca09-422f-9891-db9399fa2d3a | Oracle ID: d5482ccf-7c3f-45b8-aaa8-8f6686fe584d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLEDGLING_OSPREY,
    oracle_id = "d5482ccf-7c3f-45b8-aaa8-8f6686fe584d",
    scryfall_id = "8cd46bfa-ca09-422f-9891-db9399fa2d3a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fledgling Osprey",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::This, Filter::Enchanted]),
        Modifier::AddKeyword(KeywordSet::FLYING),
    )],
);
