//! Rush of Knowledge — {4}{U} — Sorcery
//! Oracle: Draw cards equal to the greatest mana value among permanents you control.
//! Set: 2XM #66 — Double Masters | Scryfall ID: ccca07a9-5c24-490f-8871-b125cb337e2c | Oracle ID: 34282c99-68ef-4855-b683-7bb205fa6374
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUSH_OF_KNOWLEDGE,
    oracle_id = "34282c99-68ef-4855-b683-7bb205fa6374",
    scryfall_id = "ccca07a9-5c24-490f-8871-b125cb337e2c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rush of Knowledge",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
