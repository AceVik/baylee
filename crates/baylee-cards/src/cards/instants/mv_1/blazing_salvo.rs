//! Blazing Salvo — {R} — Instant
//! Oracle: Blazing Salvo deals 3 damage to target creature unless that creature's controller has Blazing Salvo deal 5 damage to them.
//! Set: DDK #58 — Duel Decks: Sorin vs. Tibalt | Scryfall ID: 87ce6584-0a21-43d0-a2fe-cbdea564927a | Oracle ID: e6681d22-821a-47c8-9c0d-042a7d44f843
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLAZING_SALVO,
    oracle_id = "e6681d22-821a-47c8-9c0d-042a7d44f843",
    scryfall_id = "87ce6584-0a21-43d0-a2fe-cbdea564927a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blazing Salvo",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
