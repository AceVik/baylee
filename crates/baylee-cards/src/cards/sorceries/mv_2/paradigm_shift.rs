//! Paradigm Shift — {1}{U} — Sorcery
//! Oracle: Exile all cards from your library. Then shuffle your graveyard into your library.
//! Set: WTH #46 — Weatherlight | Scryfall ID: e64a17a8-091d-4029-908e-31d6a050b479 | Oracle ID: 7179c2c5-c59c-4793-a3b5-28d26d755157
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PARADIGM_SHIFT,
    oracle_id = "7179c2c5-c59c-4793-a3b5-28d26d755157",
    scryfall_id = "e64a17a8-091d-4029-908e-31d6a050b479",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Paradigm Shift",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
