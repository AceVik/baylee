//! Grip of Amnesia — {1}{U} — Instant
//! Oracle: Counter target spell unless its controller exiles all cards from their graveyard.
//! Oracle: Draw a card.
//! Set: JUD #41 — Judgment | Scryfall ID: 43dc7e2a-5b9b-4f0f-8b2e-a7c7f847e1f1 | Oracle ID: 0144d9bb-d3cb-4c23-b133-6653d7c3a76d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRIP_OF_AMNESIA,
    oracle_id = "0144d9bb-d3cb-4c23-b133-6653d7c3a76d",
    scryfall_id = "43dc7e2a-5b9b-4f0f-8b2e-a7c7f847e1f1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Grip of Amnesia",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
