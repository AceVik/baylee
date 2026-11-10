//! Ceremonial Guard — {2}{R} — Creature — Human Soldier
//! Oracle: When this creature attacks or blocks, destroy it at end of combat.
//! Set: MMQ #182 — Mercadian Masques | Scryfall ID: 4a6c69d1-5295-419f-bb8f-af826bf92cb3 | Oracle ID: 3d8aae86-266f-4bfd-a058-0fe12b5ea354
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEREMONIAL_GUARD,
    oracle_id = "3d8aae86-266f-4bfd-a058-0fe12b5ea354",
    scryfall_id = "4a6c69d1-5295-419f-bb8f-af826bf92cb3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ceremonial Guard",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        ),
    ],
);
