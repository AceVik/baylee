//! Shivan Raptor — {2}{R} — Creature — Dinosaur
//! Oracle: First strike, haste
//! Oracle: Echo {2}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #215 — Urza's Saga | Scryfall ID: 0fc45153-3cb1-43bc-b694-06f6a74b3eb7 | Oracle ID: bc01d36b-dea3-47d4-afdf-0dbbba7ca7b2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIVAN_RAPTOR,
    oracle_id = "bc01d36b-dea3-47d4-afdf-0dbbba7ca7b2",
    scryfall_id = "0fc45153-3cb1-43bc-b694-06f6a74b3eb7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shivan Raptor",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(3),
        toughness = Some(1),
        keywords = KeywordSet::FIRST_STRIKE.union(KeywordSet::HASTE),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{2}{R}")
    },],
);
