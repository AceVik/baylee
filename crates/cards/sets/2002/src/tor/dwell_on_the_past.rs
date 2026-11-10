//! Dwell on the Past — {G} — Sorcery
//! Oracle: Target player shuffles up to four target cards from their graveyard into their library.
//! Set: TOR #124 — Torment | Scryfall ID: 77ac905a-839b-4fb5-8c17-7fd6c2c0c492 | Oracle ID: 08f361c5-bcb5-4a9b-9610-70609cdc047f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWELL_ON_THE_PAST,
    oracle_id = "08f361c5-bcb5-4a9b-9610-70609cdc047f",
    scryfall_id = "77ac905a-839b-4fb5-8c17-7fd6c2c0c492",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Dwell on the Past",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
