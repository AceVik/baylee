//! Fatigue — {1}{U} — Sorcery
//! Oracle: Target player skips their next draw step.
//! Set: UDS #32 — Urza's Destiny | Scryfall ID: 660fb109-dd65-4410-99b9-a2a14f8ea202 | Oracle ID: 0a88dcb6-a391-408f-8bfc-7b5b2cc34267
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FATIGUE,
    oracle_id = "0a88dcb6-a391-408f-8bfc-7b5b2cc34267",
    scryfall_id = "660fb109-dd65-4410-99b9-a2a14f8ea202",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Fatigue",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
