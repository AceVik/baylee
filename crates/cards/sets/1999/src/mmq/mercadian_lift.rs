//! Mercadian Lift — {2} — Artifact
//! Oracle: {1}, {T}: Put a winch counter on this artifact.
//! Oracle: {T}, Remove X winch counters from this artifact: You may put a creature card with mana value X from your hand onto the battlefield.
//! Set: MMQ #306 — Mercadian Masques | Scryfall ID: 395a1a8a-785f-442b-8e95-8b4ca44af2a3 | Oracle ID: 88c3bb73-c765-4e1c-b4b9-97eaab02182f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MERCADIAN_LIFT,
    oracle_id = "88c3bb73-c765-4e1c-b4b9-97eaab02182f",
    scryfall_id = "395a1a8a-785f-442b-8e95-8b4ca44af2a3",
    faces = &[face!(
        name = "Mercadian Lift",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
