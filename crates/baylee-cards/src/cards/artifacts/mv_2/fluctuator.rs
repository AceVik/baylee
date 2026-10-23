//! Fluctuator — {2} — Artifact
//! Oracle: Cycling abilities you activate cost {2} less to activate.
//! Set: USG #295 — Urza's Saga | Scryfall ID: 92078408-e0e4-443e-b0fd-aac0ac651f46 | Oracle ID: 5ed0c7cc-9969-4387-90f8-3084dd18026b
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLUCTUATOR,
    oracle_id = "5ed0c7cc-9969-4387-90f8-3084dd18026b",
    scryfall_id = "92078408-e0e4-443e-b0fd-aac0ac651f46",
    coverage = Coverage::Partial(
        "reducing the activation cost of cycling abilities you activate from your hand by {2} is not in Modifier::AbilitiesCostLess; enters as a {2} artifact with no effect",
    ),
    faces = &[face!(
        name = "Fluctuator",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[],
);
