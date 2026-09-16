//! Expressive Iteration — {U}{R} — Sorcery
//! Oracle: Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.
//! Set: MSC #183 — Marvel Super Heroes Commander | Scryfall ID: 96f866ec-a163-4095-b088-d93fa0b6f6f7 | Oracle ID: c7aecca5-2f67-4245-ab2d-e723d8b23a67
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXPRESSIVE_ITERATION,
    oracle_id = "c7aecca5-2f67-4245-ab2d-e723d8b23a67",
    scryfall_id = "96f866ec-a163-4095-b088-d93fa0b6f6f7",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Expressive Iteration",
        mana_cost = mana!("{U}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
