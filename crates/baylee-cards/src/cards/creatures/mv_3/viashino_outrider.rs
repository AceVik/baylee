//! Viashino Outrider — {2}{R} — Creature — Lizard
//! Oracle: Echo {2}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #223 — Urza's Saga | Scryfall ID: 26ba659c-7e0f-4d8b-b91c-3c0725102ba2 | Oracle ID: 3e81c22e-52ec-4142-9a98-88681d04ce60
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_OUTRIDER,
    oracle_id = "3e81c22e-52ec-4142-9a98-88681d04ce60",
    scryfall_id = "26ba659c-7e0f-4d8b-b91c-3c0725102ba2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Outrider",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{2}{R}"),
    }],
);
