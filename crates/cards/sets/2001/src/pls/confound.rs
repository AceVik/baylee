//! Confound — {1}{U} — Instant
//! Oracle: Counter target spell that targets a creature.
//! Oracle: Draw a card.
//! Set: PLS #22 — Planeshift | Scryfall ID: 4f3b7d39-ce98-48e2-b2bf-0d55b4d3102b | Oracle ID: ca8a0e81-f885-47ee-82bb-fb25ec61bb1c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONFOUND,
    oracle_id = "ca8a0e81-f885-47ee-82bb-fb25ec61bb1c",
    scryfall_id = "4f3b7d39-ce98-48e2-b2bf-0d55b4d3102b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Confound",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
