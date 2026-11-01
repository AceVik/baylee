//! Predict — {1}{U} — Instant
//! Oracle: Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, you draw two cards. Otherwise, you draw a card.
//! Set: C18 #98 — Commander 2018 | Scryfall ID: 349ac7e6-af38-4dc3-abfe-369564c75630 | Oracle ID: 72a7faa0-cb0c-4841-84f3-e340b19fe045
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PREDICT,
    oracle_id = "72a7faa0-cb0c-4841-84f3-e340b19fe045",
    scryfall_id = "349ac7e6-af38-4dc3-abfe-369564c75630",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Predict",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
