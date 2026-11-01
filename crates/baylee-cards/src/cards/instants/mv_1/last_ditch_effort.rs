//! Last-Ditch Effort — {R} — Instant
//! Oracle: Sacrifice any number of creatures. Last-Ditch Effort deals that much damage to any target.
//! Set: ULG #83 — Urza's Legacy | Scryfall ID: 295f7fe0-0681-4b25-807f-30ed70ec78d5 | Oracle ID: cdd8d7cf-28f2-4a4d-a373-40d48bff615b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAST_DITCH_EFFORT,
    oracle_id = "cdd8d7cf-28f2-4a4d-a373-40d48bff615b",
    scryfall_id = "295f7fe0-0681-4b25-807f-30ed70ec78d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Last-Ditch Effort",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
