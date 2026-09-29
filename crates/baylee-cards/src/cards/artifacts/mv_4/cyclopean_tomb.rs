//! Cyclopean Tomb — {4} — Artifact
//! Oracle: {2}, {T}: Put a mire counter on target non-Swamp land. That land is a Swamp for as long as it has a mire counter on it. Activate only during your upkeep.
//! Oracle: When this artifact is put into a graveyard from the battlefield, at the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.
//! Set: ME4 #195 — Masters Edition IV | Scryfall ID: 7c77c7c1-39a5-4049-8e2c-3561b046152e | Oracle ID: 1edee40f-d153-4d67-aee7-ef08e11e4a79
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CYCLOPEAN_TOMB,
    oracle_id = "1edee40f-d153-4d67-aee7-ef08e11e4a79",
    scryfall_id = "7c77c7c1-39a5-4049-8e2c-3561b046152e",
    faces = &[face!(
        name = "Cyclopean Tomb",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
