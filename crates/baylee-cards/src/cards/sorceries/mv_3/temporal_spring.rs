//! Temporal Spring — {1}{G}{U} — Sorcery
//! Oracle: Put target permanent on top of its owner's library.
//! Set: APC #125 — Apocalypse | Scryfall ID: b584dfd1-a56c-406e-8504-47ea136dc102 | Oracle ID: 1d7a4e9c-b3c9-421e-b5f4-82d51d7d5125
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPORAL_SPRING,
    oracle_id = "1d7a4e9c-b3c9-421e-b5f4-82d51d7d5125",
    scryfall_id = "b584dfd1-a56c-406e-8504-47ea136dc102",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Temporal Spring",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
