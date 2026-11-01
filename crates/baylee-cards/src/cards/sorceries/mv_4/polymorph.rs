//! Polymorph — {3}{U} — Sorcery
//! Oracle: Destroy target creature. It can't be regenerated. Its controller reveals cards from the top of their library until they reveal a creature card. The player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.
//! Set: M10 #67 — Magic 2010 | Scryfall ID: 83d274f4-452b-4ff2-b026-83667e9ba98f | Oracle ID: 039115f2-6322-4674-bc64-f21883ed375a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POLYMORPH,
    oracle_id = "039115f2-6322-4674-bc64-f21883ed375a",
    scryfall_id = "83d274f4-452b-4ff2-b026-83667e9ba98f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Polymorph",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
