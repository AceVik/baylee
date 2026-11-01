//! Alloy Golem — {6} — Artifact Creature — Golem
//! Oracle: As this creature enters, choose a color.
//! Oracle: This creature is the chosen color. (It's still an artifact.)
//! Set: INV #297 — Invasion | Scryfall ID: 1fb6d6a1-9d71-405b-9c93-1a7f06c67abd | Oracle ID: 410b7faf-365c-4b73-a64d-bddbe49cfded
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALLOY_GOLEM,
    oracle_id = "410b7faf-365c-4b73-a64d-bddbe49cfded",
    scryfall_id = "1fb6d6a1-9d71-405b-9c93-1a7f06c67abd",
    faces = &[face!(
        name = "Alloy Golem",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
