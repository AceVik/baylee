//! Wall of Tears — {1}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature blocks a creature, return that creature to its owner's hand at end of combat.
//! Set: STH #50 — Stronghold | Scryfall ID: 4638d815-fb86-490c-84d6-777a1c6131d6 | Oracle ID: 5825d718-6a32-4f4c-9b56-1a50051cfe28
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_TEARS,
    oracle_id = "5825d718-6a32-4f4c-9b56-1a50051cfe28",
    scryfall_id = "4638d815-fb86-490c-84d6-777a1c6131d6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Wall of Tears",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&Filter::CREATURE),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::bounce(TargetSpec::EventObject)],
        }],
    ),],
);
