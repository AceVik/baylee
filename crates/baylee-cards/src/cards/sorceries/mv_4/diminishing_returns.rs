//! Diminishing Returns — {2}{U}{U} — Sorcery
//! Oracle: Each player shuffles their hand and graveyard into their library. You exile the top ten cards of your library. Then each player draws up to seven cards.
//! Set: EMA #46 — Eternal Masters | Scryfall ID: f9ed455f-95cd-48ca-8b0d-4db259347bb6 | Oracle ID: f9b02a7c-c122-4282-90e2-81fcfa663193
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIMINISHING_RETURNS,
    oracle_id = "f9b02a7c-c122-4282-90e2-81fcfa663193",
    scryfall_id = "f9ed455f-95cd-48ca-8b0d-4db259347bb6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Diminishing Returns",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
