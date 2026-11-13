//! Blatant Thievery — {4}{U}{U}{U} — Sorcery
//! Oracle: For each opponent, gain control of target permanent that player controls.
//! Set: C15 #87 — Commander 2015 | Scryfall ID: 0210b3b4-d253-464f-b533-b79bf67d47a4 | Oracle ID: 8bb51dd2-cc62-48ea-ae8e-52351073a8c9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLATANT_THIEVERY,
    oracle_id = "8bb51dd2-cc62-48ea-ae8e-52351073a8c9",
    scryfall_id = "0210b3b4-d253-464f-b533-b79bf67d47a4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Blatant Thievery",
        mana_cost = mana!("{4}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
