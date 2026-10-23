//! Meddle — {1}{U} — Instant
//! Oracle: If target spell has only one target and that target is a creature, change that spell's target to another creature.
//! Set: ONS #92 — Onslaught | Scryfall ID: 685edfe8-9770-47c6-95fb-0816f3126f04 | Oracle ID: 67d6fbcb-2df7-4b5a-8464-158f6b720a2e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEDDLE,
    oracle_id = "67d6fbcb-2df7-4b5a-8464-158f6b720a2e",
    scryfall_id = "685edfe8-9770-47c6-95fb-0816f3126f04",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Meddle",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
