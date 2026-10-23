//! Skyshroud Condor — {1}{U} — Creature — Bird
//! Oracle: Cast this spell only if you've cast another spell this turn.
//! Oracle: Flying
//! Set: TMP #88 — Tempest | Scryfall ID: a7d05ef5-c046-4929-b59d-988f0313a645 | Oracle ID: c7c4c67e-c4db-416b-b2e8-f0e59aa80bba
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKYSHROUD_CONDOR,
    oracle_id = "c7c4c67e-c4db-416b-b2e8-f0e59aa80bba",
    scryfall_id = "a7d05ef5-c046-4929-b59d-988f0313a645",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skyshroud Condor",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[spell!(
        &[],
        condition = Some(Condition::Not(&Condition::YouCastNoSpellThisTurn))
    )],
);
