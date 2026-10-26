//! Clear the Land — {2}{G} — Sorcery
//! Oracle: Each player reveals the top five cards of their library, puts all land cards revealed this way onto the battlefield tapped, and exiles the rest.
//! Set: MMQ #235 — Mercadian Masques | Scryfall ID: b87f3579-f314-4207-a02c-14e9cb269b47 | Oracle ID: 4604c729-8d2d-4b2b-a95c-5245563637e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLEAR_THE_LAND,
    oracle_id = "4604c729-8d2d-4b2b-a95c-5245563637e6",
    scryfall_id = "b87f3579-f314-4207-a02c-14e9cb269b47",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Clear the Land",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
