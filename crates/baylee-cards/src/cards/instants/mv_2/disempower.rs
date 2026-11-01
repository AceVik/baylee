//! Disempower — {1}{W} — Instant
//! Oracle: Put target artifact or enchantment on top of its owner's library.
//! Set: MIR #9 — Mirage | Scryfall ID: 3a86a2ae-aaf3-4d4d-ae06-ec3d4a539550 | Oracle ID: ff9f4bfe-1585-4eaa-943c-1e85ed79a820
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISEMPOWER,
    oracle_id = "ff9f4bfe-1585-4eaa-943c-1e85ed79a820",
    scryfall_id = "3a86a2ae-aaf3-4d4d-ae06-ec3d4a539550",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Disempower",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
