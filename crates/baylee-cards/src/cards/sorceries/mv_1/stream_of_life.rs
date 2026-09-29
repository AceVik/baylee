//! Stream of Life — {X}{G} — Sorcery
//! Oracle: Target player gains X life.
//! Set: 9ED #272 — Ninth Edition | Scryfall ID: 341aa1b2-e600-4580-b0cd-e1582b75dc81 | Oracle ID: 9eb2912d-2130-49f2-9529-b58fa5a97a15
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STREAM_OF_LIFE,
    oracle_id = "9eb2912d-2130-49f2-9529-b58fa5a97a15",
    scryfall_id = "341aa1b2-e600-4580-b0cd-e1582b75dc81",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Stream of Life",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
