//! Sprouting Vines — {2}{G} — Instant
//! Oracle: Search your library for a basic land card, reveal that card, put it into your hand, then shuffle.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn.)
//! Set: C13 #173 — Commander 2013 | Scryfall ID: b16ba7be-47e6-4c64-918d-bec60abccaa3 | Oracle ID: a38ad1aa-fc12-49be-b9f7-fe287597a5ca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPROUTING_VINES,
    oracle_id = "a38ad1aa-fc12-49be-b9f7-fe287597a5ca",
    scryfall_id = "b16ba7be-47e6-4c64-918d-bec60abccaa3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sprouting Vines",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
