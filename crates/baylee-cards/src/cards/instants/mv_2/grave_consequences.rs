//! Grave Consequences — {1}{B} — Instant
//! Oracle: Each player may exile any number of cards from their graveyard. Then each player loses 1 life for each card in their graveyard.
//! Oracle: Draw a card.
//! Set: JUD #67 — Judgment | Scryfall ID: 9ad5f9f2-282a-4ee0-a259-cc24404ddf6f | Oracle ID: 435c8d98-9fa3-40d8-9b7c-32474099e08e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRAVE_CONSEQUENCES,
    oracle_id = "435c8d98-9fa3-40d8-9b7c-32474099e08e",
    scryfall_id = "9ad5f9f2-282a-4ee0-a259-cc24404ddf6f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Grave Consequences",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
