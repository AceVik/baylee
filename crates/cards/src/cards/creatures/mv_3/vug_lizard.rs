//! Vug Lizard — {1}{R}{R} — Creature — Lizard
//! Oracle: Mountainwalk (This creature can't be blocked as long as defending player controls a Mountain.)
//! Oracle: Echo {1}{R}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #227 — Urza's Saga | Scryfall ID: 39bbb1f6-f3c4-4e11-bb71-91ea31797d1e | Oracle ID: ac9ec838-fbad-4f01-846b-3084e1d37a44
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VUG_LIZARD,
    oracle_id = "ac9ec838-fbad-4f01-846b-3084e1d37a44",
    scryfall_id = "39bbb1f6-f3c4-4e11-bb71-91ea31797d1e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vug Lizard",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(3),
        toughness = Some(4),
        keywords = KeywordSet::MOUNTAINWALK,
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{1}{R}{R}"),
    }],
);
