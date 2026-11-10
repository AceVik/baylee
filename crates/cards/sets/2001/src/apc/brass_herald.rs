//! Brass Herald — {6} — Artifact Creature — Golem
//! Oracle: As this creature enters, choose a creature type.
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all creature cards of the chosen type revealed this way into your hand and the rest on the bottom of your library in any order.
//! Oracle: Creatures of the chosen type get +1/+1.
//! Set: CMR #301 — Commander Legends | Scryfall ID: 7c0380a4-225f-4a80-8c8b-d0f7e54f1626 | Oracle ID: ea9b2e3d-2de4-4cde-b233-6d0ba5797c20
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRASS_HERALD,
    oracle_id = "ea9b2e3d-2de4-4cde-b233-6d0ba5797c20",
    scryfall_id = "7c0380a4-225f-4a80-8c8b-d0f7e54f1626",
    faces = &[face!(
        name = "Brass Herald",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
