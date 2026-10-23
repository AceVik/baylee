//! Damping Engine — {4} — Artifact
//! Oracle: A player who controls more permanents than each other player can't play lands or cast artifact, creature, or enchantment spells. That player may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.
//! Set: ULG #124 — Urza's Legacy | Scryfall ID: 87841977-75ff-49c3-b832-3f0cf48b50b2 | Oracle ID: 3584a055-5220-4721-89f8-de33f779122f
// PARTIAL: comparing each player's permanent count and the sacrifice-to-ignore special action are not supported; enters as an artifact.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAMPING_ENGINE,
    oracle_id = "3584a055-5220-4721-89f8-de33f779122f",
    scryfall_id = "87841977-75ff-49c3-b832-3f0cf48b50b2",
    coverage = Coverage::Partial(
        "comparing each player's permanent count and the sacrifice-to-ignore special action are not supported; enters as an artifact",
    ),
    faces = &[face!(
        name = "Damping Engine",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);
