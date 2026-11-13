//! Vile Consumption — {1}{U}{B} — Enchantment
//! Oracle: All creatures have "At the beginning of your upkeep, sacrifice this creature unless you pay 1 life."
//! Set: INV #285 — Invasion | Scryfall ID: 7f7e5716-77f3-45d2-a40a-f5bf500f6ad7 | Oracle ID: b6a005db-e7e2-4b88-bc8a-c257bcc31cf5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VILE_CONSUMPTION,
    oracle_id = "b6a005db-e7e2-4b88-bc8a-c257bcc31cf5",
    scryfall_id = "7f7e5716-77f3-45d2-a40a-f5bf500f6ad7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Vile Consumption",
        mana_cost = mana!("{1}{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
