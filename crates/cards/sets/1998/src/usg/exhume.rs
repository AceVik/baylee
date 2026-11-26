//! Exhume — {1}{B} — Sorcery
//! Oracle: Each player puts a creature card from their graveyard onto the battlefield.
//! Set: JMP #230 — Jumpstart | Scryfall ID: 5f1cdcba-a04a-4a2f-8bc1-0dd7fa03754d | Oracle ID: fbe61f74-1b3c-4e12-8758-7029872c9ff1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXHUME,
    oracle_id = "fbe61f74-1b3c-4e12-8758-7029872c9ff1",
    scryfall_id = "5f1cdcba-a04a-4a2f-8bc1-0dd7fa03754d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Exhume",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
