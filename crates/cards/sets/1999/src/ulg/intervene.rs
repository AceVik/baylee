//! Intervene — {U} — Instant
//! Oracle: Counter target spell that targets a creature.
//! Set: ULG #33 — Urza's Legacy | Scryfall ID: 4b0e3894-5dfe-4d03-9996-eebf96c58168 | Oracle ID: 380b386f-90c8-45c5-a611-1ed6803b52d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTERVENE,
    oracle_id = "380b386f-90c8-45c5-a611-1ed6803b52d2",
    scryfall_id = "4b0e3894-5dfe-4d03-9996-eebf96c58168",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Intervene",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
