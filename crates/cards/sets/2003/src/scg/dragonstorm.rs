//! Dragonstorm — {8}{R} — Sorcery
//! Oracle: Search your library for a Dragon permanent card, put it onto the battlefield, then shuffle.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn.)
//! Set: MMA #111 — Modern Masters | Scryfall ID: 230cd568-f7ed-4571-a609-36522add91d0 | Oracle ID: d74c3871-018a-4450-a613-f67703bb0d72
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAGONSTORM,
    oracle_id = "d74c3871-018a-4450-a613-f67703bb0d72",
    scryfall_id = "230cd568-f7ed-4571-a609-36522add91d0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dragonstorm",
        mana_cost = mana!("{8}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
