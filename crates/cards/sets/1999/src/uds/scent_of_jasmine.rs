//! Scent of Jasmine — {W} — Instant
//! Oracle: Reveal any number of white cards in your hand. You gain 2 life for each card revealed this way.
//! Set: UDS #17 — Urza's Destiny | Scryfall ID: c0dae0cf-9696-498c-8d28-dc8c239faec7 | Oracle ID: 3a532c2b-a08c-46ab-86f7-c6dee0d6985f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCENT_OF_JASMINE,
    oracle_id = "3a532c2b-a08c-46ab-86f7-c6dee0d6985f",
    scryfall_id = "c0dae0cf-9696-498c-8d28-dc8c239faec7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Scent of Jasmine",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
