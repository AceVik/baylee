//! Citanul Centaurs — {3}{G} — Creature — Centaur
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)
//! Oracle: Echo {3}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #243 — Urza's Saga | Scryfall ID: 5a3ac987-7906-4159-a007-ed409baea9d7 | Oracle ID: 831487cf-a377-4c9b-920d-c5bc68f17f24
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CITANUL_CENTAURS,
    oracle_id = "831487cf-a377-4c9b-920d-c5bc68f17f24",
    scryfall_id = "5a3ac987-7906-4159-a007-ed409baea9d7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::SHROUD,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Citanul Centaurs",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR],
        power = Some(6),
        toughness = Some(3),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{3}{G}"),
    }],
);
