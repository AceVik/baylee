//! Ertai's Meddling — {X}{U} — Instant
//! Oracle: X can't be 0.
//! Oracle: Target spell's controller exiles it with X delay counters on it.
//! Oracle: At the beginning of each of that player's upkeeps, if that card is exiled, remove a delay counter from it. If the card has no delay counters on it, the player puts it onto the stack as a copy of the original spell.
//! Set: TMP #61 — Tempest | Scryfall ID: 35c7e7fa-1493-4ef8-9cdb-b02b07a1ad85 | Oracle ID: 609c529a-b3b3-46d5-b9d0-ae0117507d6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ERTAI_S_MEDDLING,
    oracle_id = "609c529a-b3b3-46d5-b9d0-ae0117507d6e",
    scryfall_id = "35c7e7fa-1493-4ef8-9cdb-b02b07a1ad85",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ertai's Meddling",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
