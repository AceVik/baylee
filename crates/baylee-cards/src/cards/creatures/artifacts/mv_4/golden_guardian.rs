//! Golden Guardian // Gold-Forge Garrison — {4} — Artifact Creature — Golem // Land
//! Set: RIX #179 — Rivals of Ixalan | Scryfall ID: 397ba02d-f347-46f7-b028-dd4ba55faa2f | Oracle ID: 58afb897-4d57-4b53-a5c3-b532cb3d5180
//! Face: Golden Guardian — {4} — Artifact Creature — Golem
//! Face: Gold-Forge Garrison —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 563,
    oracle_id: "58afb897-4d57-4b53-a5c3-b532cb3d5180",
    scryfall_id: "397ba02d-f347-46f7-b028-dd4ba55faa2f",
    faces: &[
    face! {
        name: "Golden Guardian",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes: &[subtypes::creature::GOLEM],
        power: Some(4),
        toughness: Some(4),
    },
    face! {
        name: "Gold-Forge Garrison",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
