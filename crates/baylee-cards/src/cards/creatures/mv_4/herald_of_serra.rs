//! Herald of Serra — {2}{W}{W} — Creature — Angel
//! Oracle: Flying, vigilance
//! Oracle: Echo {2}{W}{W} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #17 — Urza's Saga | Scryfall ID: 22a2b882-d616-495e-99f6-196031235f93 | Oracle ID: bc42f32d-e4e1-4174-904c-6058e487f963
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HERALD_OF_SERRA,
    oracle_id = "bc42f32d-e4e1-4174-904c-6058e487f963",
    scryfall_id = "22a2b882-d616-495e-99f6-196031235f93",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Herald of Serra",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{2}{W}{W}"),
    }],
);
