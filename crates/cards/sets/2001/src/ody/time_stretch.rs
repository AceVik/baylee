//! Time Stretch — {8}{U}{U} — Sorcery
//! Oracle: Target player takes two extra turns after this one.
//! Set: DMR #69 — Dominaria Remastered | Scryfall ID: 9dc791fd-d49c-4c66-b8fb-6b54ce842e90 | Oracle ID: 72e56963-a9dd-44dc-a4d3-992b4d89dd28
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_STRETCH,
    oracle_id = "72e56963-a9dd-44dc-a4d3-992b4d89dd28",
    scryfall_id = "9dc791fd-d49c-4c66-b8fb-6b54ce842e90",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Time Stretch",
        mana_cost = mana!("{8}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
