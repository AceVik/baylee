//! Walking Ballista — {X}{X} — Artifact Creature — Construct
//! Oracle: This creature enters with X +1/+1 counters on it.
//! Oracle: {4}: Put a +1/+1 counter on this creature.
//! Oracle: Remove a +1/+1 counter from this creature: It deals 1 damage to any target.
//! Set: 2XM #306 — Double Masters | Scryfall ID: 5272436e-74f0-44c4-a291-ea8ebc3f1525 | Oracle ID: 4b515bb0-f275-4400-8032-3173b799ab40
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_BALLISTA,
    oracle_id = "4b515bb0-f275-4400-8032-3173b799ab40",
    scryfall_id = "5272436e-74f0-44c4-a291-ea8ebc3f1525",
    faces = &[face!(
        name = "Walking Ballista",
        mana_cost = mana!("{X}{X}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
