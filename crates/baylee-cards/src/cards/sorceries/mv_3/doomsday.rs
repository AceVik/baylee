//! Doomsday — {B}{B}{B} — Sorcery
//! Oracle: Search your library and graveyard for five cards and exile the rest. Put the chosen cards on top of your library in any order. You lose half your life, rounded up.
//! Set: MSC #796 — Marvel Super Heroes Commander | Scryfall ID: 72cd296e-e550-46bf-9378-586a24c9b2a7 | Oracle ID: 721eb5a2-d7cf-4db0-8013-ef3f596c52a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DOOMSDAY,
    oracle_id = "721eb5a2-d7cf-4db0-8013-ef3f596c52a5",
    scryfall_id = "72cd296e-e550-46bf-9378-586a24c9b2a7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Doomsday",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
