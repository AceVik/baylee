//! Faces of the Past — {2}{U} — Enchantment
//! Oracle: Whenever a creature dies, tap all untapped creatures that share a creature type with it or untap all tapped creatures that share a creature type with it.
//! Set: SCG #35 — Scourge | Scryfall ID: 0f6dc35b-eb26-498f-ae35-0e860871446e | Oracle ID: c8cd2b52-6d32-413f-9b26-2f9d436dbeae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FACES_OF_THE_PAST,
    oracle_id = "c8cd2b52-6d32-413f-9b26-2f9d436dbeae",
    scryfall_id = "0f6dc35b-eb26-498f-ae35-0e860871446e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Faces of the Past",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
