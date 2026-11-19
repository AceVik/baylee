//! Drown in the Loch — {U}{B} — Instant
//! Oracle: Choose one —
//! Oracle: • Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard.
//! Oracle: • Destroy target creature with mana value less than or equal to the number of cards in its controller's graveyard.
//! Set: CLB #842 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: eb7b9762-740b-4c1c-a411-20dbf023aea5 | Oracle ID: 0f264e5b-264e-4e97-9a8d-8ae1d6a286ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DROWN_IN_THE_LOCH,
    oracle_id = "0f264e5b-264e-4e97-9a8d-8ae1d6a286ce",
    scryfall_id = "eb7b9762-740b-4c1c-a411-20dbf023aea5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Drown in the Loch",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
