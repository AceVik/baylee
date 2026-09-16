//! Thought Monitor — {6}{U} — Artifact Creature — Construct
//! Oracle: Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)
//! Oracle: Flying
//! Oracle: When this creature enters, draw two cards.
//! Set: EOC #79 — Edge of Eternities Commander | Scryfall ID: 18a6ea89-417c-4ee0-a410-8a0067b92967 | Oracle ID: 9deded8b-cec4-4ede-a50b-131404d456d4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHT_MONITOR,
    oracle_id = "9deded8b-cec4-4ede-a50b-131404d456d4",
    scryfall_id = "18a6ea89-417c-4ee0-a410-8a0067b92967",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Monitor",
        mana_cost = mana!("{6}{U}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
