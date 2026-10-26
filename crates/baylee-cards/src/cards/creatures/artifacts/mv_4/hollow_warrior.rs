//! Hollow Warrior — {4} — Artifact Creature — Golem Warrior
//! Oracle: This creature can't attack or block unless you tap an untapped creature you control not declared as an attacking or blocking creature this combat. (This cost is paid as attackers or blockers are declared.)
//! Set: PCY #138 — Prophecy | Scryfall ID: 163add05-9ee9-4a8f-8838-3c4143ddc2f5 | Oracle ID: 18ca77d9-4740-4be4-913a-e21c9ed9e43f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HOLLOW_WARRIOR,
    oracle_id = "18ca77d9-4740-4be4-913a-e21c9ed9e43f",
    scryfall_id = "163add05-9ee9-4a8f-8838-3c4143ddc2f5",
    faces = &[face!(
        name = "Hollow Warrior",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM, subtypes::creature::WARRIOR],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
