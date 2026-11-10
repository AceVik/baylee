//! Wash Out — {3}{U} — Sorcery
//! Oracle: Return all permanents of the color of your choice to their owners' hands.
//! Set: 2X2 #67 — Double Masters 2022 | Scryfall ID: dc468f97-08f7-4103-898f-b14fa61b99bc | Oracle ID: 54748cb1-d92a-4212-ad76-417ee79b5ef1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WASH_OUT,
    oracle_id = "54748cb1-d92a-4212-ad76-417ee79b5ef1",
    scryfall_id = "dc468f97-08f7-4103-898f-b14fa61b99bc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wash Out",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
