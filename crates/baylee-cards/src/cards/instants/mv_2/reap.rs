//! Reap — {1}{G} — Instant
//! Oracle: Return up to X target cards from your graveyard to your hand, where X is the number of black permanents target opponent controls as you cast this spell.
//! Set: TMP #247 — Tempest | Scryfall ID: 229f8a3d-d1a5-46d7-9b1b-e165397e6579 | Oracle ID: 7fcb8472-1e77-4dcd-b18f-8abf8806da08
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REAP,
    oracle_id = "7fcb8472-1e77-4dcd-b18f-8abf8806da08",
    scryfall_id = "229f8a3d-d1a5-46d7-9b1b-e165397e6579",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Reap",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
