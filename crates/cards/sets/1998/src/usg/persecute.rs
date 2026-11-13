//! Persecute — {2}{B}{B} — Sorcery
//! Oracle: Choose a color. Target player reveals their hand and discards all cards of that color.
//! Set: USG #146 — Urza's Saga | Scryfall ID: 4f8e2be6-124d-412e-be34-aa4495a83e02 | Oracle ID: 676c4cfa-f929-43aa-95cc-df81a596c2a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PERSECUTE,
    oracle_id = "676c4cfa-f929-43aa-95cc-df81a596c2a2",
    scryfall_id = "4f8e2be6-124d-412e-be34-aa4495a83e02",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Persecute",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
