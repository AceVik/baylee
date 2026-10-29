//! Equal Treatment — {1}{W} — Instant
//! Oracle: If any source would deal 1 or more damage to a permanent or player this turn, it deals 2 damage to that permanent or player instead.
//! Oracle: Draw a card.
//! Set: TOR #4 — Torment | Scryfall ID: 310cb525-9299-471e-b310-392353b25472 | Oracle ID: beef71f4-f607-4718-9fde-98e8d7871ce9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EQUAL_TREATMENT,
    oracle_id = "beef71f4-f607-4718-9fde-98e8d7871ce9",
    scryfall_id = "310cb525-9299-471e-b310-392353b25472",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Equal Treatment",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
