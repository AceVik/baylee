//! Wall of Junk — {2} — Artifact Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: When this creature blocks, return it to its owner's hand at end of combat. (Return it only if it's on the battlefield.)
//! Set: USG #315 — Urza's Saga | Scryfall ID: cd3f88b3-9796-4d4d-af8b-543f0d280e77 | Oracle ID: 22b0de2d-47a8-4269-ba21-4bc2b4b10b40
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_JUNK,
    oracle_id = "22b0de2d-47a8-4269-ba21-4bc2b4b10b40",
    scryfall_id = "cd3f88b3-9796-4d4d-af8b-543f0d280e77",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Junk",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(7),
        keywords = KeywordSet::DEFENDER,
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::bounce(TargetSpec::EventObject)],
        }],
    )],
);
