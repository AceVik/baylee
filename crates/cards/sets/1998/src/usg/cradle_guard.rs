//! Cradle Guard — {1}{G}{G} — Creature — Treefolk
//! Oracle: Trample
//! Oracle: Echo {1}{G}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #245 — Urza's Saga | Scryfall ID: 8b659c1c-cc0b-40f0-87b4-aeddb44dfac5 | Oracle ID: 846c0867-6f0e-4e44-8f45-8f9720b62065
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRADLE_GUARD,
    oracle_id = "846c0867-6f0e-4e44-8f45-8f9720b62065",
    scryfall_id = "8b659c1c-cc0b-40f0-87b4-aeddb44dfac5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cradle Guard",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{1}{G}{G}"),
    }],
);
