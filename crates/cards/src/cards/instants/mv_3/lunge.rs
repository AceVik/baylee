//! Lunge — {2}{R} — Instant
//! Oracle: Lunge deals 2 damage to target creature and 2 damage to target player or planeswalker.
//! Set: MMQ #203 — Mercadian Masques | Scryfall ID: e9e43349-429c-43f7-b808-c4bf37370a9f | Oracle ID: 5e642112-9592-475f-ba03-d71544270cfa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LUNGE,
    oracle_id = "5e642112-9592-475f-ba03-d71544270cfa",
    scryfall_id = "e9e43349-429c-43f7-b808-c4bf37370a9f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lunge",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
