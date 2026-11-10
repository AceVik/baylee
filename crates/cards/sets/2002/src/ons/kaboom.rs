//! Kaboom! — {4}{R} — Sorcery
//! Oracle: Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal a nonland card, Kaboom! deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order.
//! Set: ONS #214 — Onslaught | Scryfall ID: 1e81e5fc-0e18-4dd8-a505-aa7dba8521a8 | Oracle ID: c92802ff-484c-4d55-aba4-c3a000264113
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KABOOM,
    oracle_id = "c92802ff-484c-4d55-aba4-c3a000264113",
    scryfall_id = "1e81e5fc-0e18-4dd8-a505-aa7dba8521a8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kaboom!",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
