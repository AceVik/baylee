//! Manipulate Fate — {1}{U} — Sorcery
//! Oracle: Search your library for three cards, exile them, then shuffle.
//! Oracle: Draw a card.
//! Set: INV #60 — Invasion | Scryfall ID: 5bb52acb-dedb-4ed6-a6da-8c036f2b2958 | Oracle ID: 541f13aa-a1ed-453a-b719-82daef1be607
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANIPULATE_FATE,
    oracle_id = "541f13aa-a1ed-453a-b719-82daef1be607",
    scryfall_id = "5bb52acb-dedb-4ed6-a6da-8c036f2b2958",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Manipulate Fate",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
