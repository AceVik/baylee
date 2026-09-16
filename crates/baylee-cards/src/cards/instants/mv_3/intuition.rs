//! Intuition — {2}{U} — Instant
//! Oracle: Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.
//! Set: TPR #54 — Tempest Remastered | Scryfall ID: b13e73a5-067d-4dbd-9c98-34a0db6140de | Oracle ID: 3c9faba7-f2d3-4978-be94-020dc8003dc0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTUITION,
    oracle_id = "3c9faba7-f2d3-4978-be94-020dc8003dc0",
    scryfall_id = "b13e73a5-067d-4dbd-9c98-34a0db6140de",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Intuition",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
