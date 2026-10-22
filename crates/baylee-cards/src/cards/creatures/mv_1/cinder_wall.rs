//! Cinder Wall — {R} — Creature — Wall
//! Oracle: Defender
//! Oracle: When this creature blocks, destroy it at end of combat.
//! Set: CNS #140 — Conspiracy | Scryfall ID: 4dfcc240-9010-4e0c-afda-7f70c42da7d2 | Oracle ID: 16128b90-9b81-476e-8999-3ef498728c39
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CINDER_WALL,
    oracle_id = "16128b90-9b81-476e-8999-3ef498728c39",
    scryfall_id = "4dfcc240-9010-4e0c-afda-7f70c42da7d2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cinder Wall",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::DEFENDER,
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::destroy(TargetSpec::EventObject)],
        }],
    )],
);
