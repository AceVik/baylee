//! Search for Survivors — {2}{R} — Sorcery
//! Oracle: Reorder your graveyard at random. An opponent chooses a card at random in your graveyard. If it's a creature card, put it onto the battlefield. Otherwise, exile it.
//! Set: PCY #102 — Prophecy | Scryfall ID: 2f19a1b5-48ba-44a9-b91f-2f628b223ffb | Oracle ID: 02fd4da6-6c6e-4143-a915-495c668de148
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEARCH_FOR_SURVIVORS,
    oracle_id = "02fd4da6-6c6e-4143-a915-495c668de148",
    scryfall_id = "2f19a1b5-48ba-44a9-b91f-2f628b223ffb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Search for Survivors",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
