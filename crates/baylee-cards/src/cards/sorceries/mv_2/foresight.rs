//! Foresight — {1}{U} — Sorcery
//! Oracle: Search your library for three cards, exile them, then shuffle.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #29a — Alliances | Scryfall ID: a12e624c-8879-4e60-a1be-286abc5e0106 | Oracle ID: c9640b04-9751-46d5-b7bf-5b8a8ecf145f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORESIGHT,
    oracle_id = "c9640b04-9751-46d5-b7bf-5b8a8ecf145f",
    scryfall_id = "a12e624c-8879-4e60-a1be-286abc5e0106",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Foresight",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
