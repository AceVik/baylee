//! Junk Golem — {4} — Artifact Creature — Golem
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you remove a +1/+1 counter from it.
//! Oracle: {1}, Discard a card: Put a +1/+1 counter on this creature.
//! Set: ODY #300 — Odyssey | Scryfall ID: c9410da0-5693-456f-afa8-cb92ac176847 | Oracle ID: 044be66d-9334-4d4b-b420-b97a18698e95
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JUNK_GOLEM,
    oracle_id = "044be66d-9334-4d4b-b420-b97a18698e95",
    scryfall_id = "c9410da0-5693-456f-afa8-cb92ac176847",
    faces = &[face!(
        name = "Junk Golem",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
