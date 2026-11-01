//! Tidal Wave — {2}{U} — Instant
//! Oracle: Create a 5/5 blue Wall creature token with defender. Sacrifice it at the beginning of the next end step.
//! Set: DDT #21 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: 9898c19f-7341-4be9-b592-bac884e55c84 | Oracle ID: c5f5b7e5-af1e-4c8a-8d44-3e2152955d4e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIDAL_WAVE,
    oracle_id = "c5f5b7e5-af1e-4c8a-8d44-3e2152955d4e",
    scryfall_id = "9898c19f-7341-4be9-b592-bac884e55c84",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tidal Wave",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
