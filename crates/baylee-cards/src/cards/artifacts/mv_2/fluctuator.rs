//! Fluctuator — {2} — Artifact
//! Oracle: Cycling abilities you activate cost {2} less to activate.
//! Set: USG #295 — Urza's Saga | Scryfall ID: 92078408-e0e4-443e-b0fd-aac0ac651f46 | Oracle ID: 5ed0c7cc-9969-4387-90f8-3084dd18026b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLUCTUATOR,
    oracle_id = "5ed0c7cc-9969-4387-90f8-3084dd18026b",
    scryfall_id = "92078408-e0e4-443e-b0fd-aac0ac651f46",
    faces = &[face!(
        name = "Fluctuator",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
