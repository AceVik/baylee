//! Rebound — {1}{U} — Instant
//! Oracle: Change the target of target spell that targets only a player. The new target must be a player.
//! Set: STH #40 — Stronghold | Scryfall ID: bb6ca66e-1116-4739-8375-87af99e9bba5 | Oracle ID: 9769ed80-b546-49f5-9329-689e619dabdd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REBOUND,
    oracle_id = "9769ed80-b546-49f5-9329-689e619dabdd",
    scryfall_id = "bb6ca66e-1116-4739-8375-87af99e9bba5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rebound",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
