//! Avalanche Riders — {3}{R} — Creature — Human Nomad
//! Oracle: Haste
//! Oracle: Echo {3}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, destroy target land.
//! Set: TSB #55 — Time Spiral Timeshifted | Scryfall ID: abb19b6b-4142-4b93-85ca-3f2d8b23b4e1 | Oracle ID: c9048d35-aff7-45ae-ae97-db89397b3e74
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVALANCHE_RIDERS,
    oracle_id = "c9048d35-aff7-45ae-ae97-db89397b3e74",
    scryfall_id = "abb19b6b-4142-4b93-85ca-3f2d8b23b4e1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Avalanche Riders",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{3}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
        ),
    ],
);
