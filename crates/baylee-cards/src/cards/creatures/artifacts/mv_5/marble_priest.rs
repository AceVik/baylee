//! Marble Priest — {5} — Artifact Creature — Cleric
//! Oracle: All Walls able to block this creature do so.
//! Oracle: Prevent all combat damage that would be dealt to this creature by Walls.
//! Set: LEG #286 — Legends | Scryfall ID: 459b71d7-34c1-43b9-93ff-364f95aa4789 | Oracle ID: 96bcabee-e84e-409f-8439-80f5308d73e9
// IMPLEMENTED — every Wall able to block it must; combat damage Walls would deal to it is prevented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// A Wall creature.
const WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

card!(
    index = index::MARBLE_PRIEST,
    oracle_id = "96bcabee-e84e-409f-8439-80f5308d73e9",
    scryfall_id = "459b71d7-34c1-43b9-93ff-364f95aa4789",
    faces = &[face!(
        name = "Marble Priest",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::MustBeBlockedByAllAbleMatching(&WALL)
        ),
        static_ability!(Filter::This, Modifier::PreventCombatDamageFrom(&WALL)),
    ],
);
