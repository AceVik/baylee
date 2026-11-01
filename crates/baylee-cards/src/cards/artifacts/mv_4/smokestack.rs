//! Smokestack — {4} — Artifact
//! Oracle: At the beginning of your upkeep, you may put a soot counter on this artifact.
//! Oracle: At the beginning of each player's upkeep, that player sacrifices a permanent of their choice for each soot counter on this artifact.
//! Set: USG #309 — Urza's Saga | Scryfall ID: 6a37ab2d-094f-431e-878f-93aef0360413 | Oracle ID: fda61506-ed79-47fd-badd-639185297913
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SMOKESTACK,
    oracle_id = "fda61506-ed79-47fd-badd-639185297913",
    scryfall_id = "6a37ab2d-094f-431e-878f-93aef0360413",
    faces = &[face!(
        name = "Smokestack",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
