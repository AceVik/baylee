//! Strongarm Tactics — {1}{B} — Sorcery
//! Oracle: Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.
//! Set: ONS #173 — Onslaught | Scryfall ID: 57dcf434-5c67-440a-8b67-2df7307e92bd | Oracle ID: 04ec6f72-8b7e-487b-b075-84276cd6ecfd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STRONGARM_TACTICS,
    oracle_id = "04ec6f72-8b7e-487b-b075-84276cd6ecfd",
    scryfall_id = "57dcf434-5c67-440a-8b67-2df7307e92bd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Strongarm Tactics",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
