//! Chain of Silence — {1}{W} — Instant
//! Oracle: Prevent all damage target creature would deal this turn. That creature's controller may sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: ONS #12 — Onslaught | Scryfall ID: 9a60ac8e-11eb-433f-86f9-8e593b38c617 | Oracle ID: d7b476ed-fc50-4804-8375-488a9e2ab184
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_OF_SILENCE,
    oracle_id = "d7b476ed-fc50-4804-8375-488a9e2ab184",
    scryfall_id = "9a60ac8e-11eb-433f-86f9-8e593b38c617",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Chain of Silence",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
