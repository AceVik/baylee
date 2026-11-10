//! Karmic Justice — {2}{W} — Enchantment
//! Oracle: Whenever a spell or ability an opponent controls destroys a noncreature permanent you control, you may destroy target permanent that opponent controls.
//! Set: C15 #72 — Commander 2015 | Scryfall ID: ea141e39-8124-4d72-a154-eb76e71be1dc | Oracle ID: 9ea58595-fcb9-455c-9558-b6a3fb2b83c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KARMIC_JUSTICE,
    oracle_id = "9ea58595-fcb9-455c-9558-b6a3fb2b83c5",
    scryfall_id = "ea141e39-8124-4d72-a154-eb76e71be1dc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Karmic Justice",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
