//! Stunted Growth — {3}{G}{G} — Sorcery
//! Oracle: Target player chooses three cards from their hand and puts them on top of their library in any order.
//! Set: ME2 #179 — Masters Edition II | Scryfall ID: 413bebf6-d47c-4bd2-ad2d-9b1ae70ef066 | Oracle ID: e3480c8f-15ad-451b-bb34-b129805569e1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUNTED_GROWTH,
    oracle_id = "e3480c8f-15ad-451b-bb34-b129805569e1",
    scryfall_id = "413bebf6-d47c-4bd2-ad2d-9b1ae70ef066",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Stunted Growth",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
