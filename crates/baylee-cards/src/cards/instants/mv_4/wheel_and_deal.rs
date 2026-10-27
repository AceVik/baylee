//! Wheel and Deal — {3}{U} — Instant
//! Oracle: Any number of target opponents each discard their hands, then draw seven cards.
//! Oracle: Draw a card.
//! Set: ONS #121 — Onslaught | Scryfall ID: 61f50a1a-f3d0-4fcf-bd32-0e173b0d3247 | Oracle ID: a7c6f07c-3cc3-4271-bbc7-9f7d5bbed3e8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHEEL_AND_DEAL,
    oracle_id = "a7c6f07c-3cc3-4271-bbc7-9f7d5bbed3e8",
    scryfall_id = "61f50a1a-f3d0-4fcf-bd32-0e173b0d3247",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wheel and Deal",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
