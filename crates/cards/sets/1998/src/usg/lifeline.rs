//! Lifeline — {5} — Artifact
//! Oracle: Whenever a creature dies, if another creature is on the battlefield, return the first card to the battlefield under its owner's control at the beginning of the next end step.
//! Set: USG #299 — Urza's Saga | Scryfall ID: 40cee82f-36b2-48a9-930a-8e23cb2742fc | Oracle ID: f475b636-eb7f-4dd7-bc87-a1af07064ffe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFELINE,
    oracle_id = "f475b636-eb7f-4dd7-bc87-a1af07064ffe",
    scryfall_id = "40cee82f-36b2-48a9-930a-8e23cb2742fc",
    faces = &[face!(
        name = "Lifeline",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
