//! Evasive Action — {1}{U} — Instant
//! Oracle: Domain — Counter target spell unless its controller pays {1} for each basic land type among lands you control.
//! Set: DDE #50 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: d8fad630-bd1c-42df-86b5-cc00da28abfd | Oracle ID: 4543a99d-eefa-470d-976d-11250524ae28
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EVASIVE_ACTION,
    oracle_id = "4543a99d-eefa-470d-976d-11250524ae28",
    scryfall_id = "d8fad630-bd1c-42df-86b5-cc00da28abfd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Evasive Action",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
