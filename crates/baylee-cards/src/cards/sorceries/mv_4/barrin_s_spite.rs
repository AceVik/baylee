//! Barrin's Spite — {2}{U}{B} — Sorcery
//! Oracle: Choose two target creatures controlled by the same player. Their controller chooses and sacrifices one of them. Return the other to its owner's hand.
//! Set: INV #235 — Invasion | Scryfall ID: 6d8ec4dc-c74a-4d49-856e-95703675fe9b | Oracle ID: 68b038ef-71c4-4ca5-9821-57bf85831bb7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARRIN_S_SPITE,
    oracle_id = "68b038ef-71c4-4ca5-9821-57bf85831bb7",
    scryfall_id = "6d8ec4dc-c74a-4d49-856e-95703675fe9b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Barrin's Spite",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
