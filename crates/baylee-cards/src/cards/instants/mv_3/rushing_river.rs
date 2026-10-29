//! Rushing River — {2}{U} — Instant
//! Oracle: Kicker—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell.)
//! Oracle: Return target nonland permanent to its owner's hand. If this spell was kicked, return another target nonland permanent to its owner's hand.
//! Set: PLS #30 — Planeshift | Scryfall ID: 52ddf7bf-de9c-4657-8d5b-79869d36fa63 | Oracle ID: 6e2a695d-305a-4bbe-b302-11fdc5c2a451
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUSHING_RIVER,
    oracle_id = "6e2a695d-305a-4bbe-b302-11fdc5c2a451",
    scryfall_id = "52ddf7bf-de9c-4657-8d5b-79869d36fa63",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rushing River",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
