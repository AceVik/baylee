//! Prophetic Bolt — {3}{U}{R} — Instant
//! Oracle: Prophetic Bolt deals 4 damage to any target. Look at the top four cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order.
//! Set: 2X2 #265 — Double Masters 2022 | Scryfall ID: 94b79c31-597f-484b-a684-f1520cae314a | Oracle ID: d5e4bf5e-1a66-4315-8c82-640c27977b88
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROPHETIC_BOLT,
    oracle_id = "d5e4bf5e-1a66-4315-8c82-640c27977b88",
    scryfall_id = "94b79c31-597f-484b-a684-f1520cae314a",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Prophetic Bolt",
        mana_cost = mana!("{3}{U}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
