//! Lifetap — {U}{U} — Enchantment
//! Oracle: Whenever a Forest an opponent controls becomes tapped, you gain 1 life.
//! Set: 5ED #99 — Fifth Edition | Scryfall ID: af066a2d-d357-45a7-90d8-2c34b6d0ebf8 | Oracle ID: 52ac09af-2aa7-4d80-be26-3e6a6efd5c23
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFETAP,
    oracle_id = "52ac09af-2aa7-4d80-be26-3e6a6efd5c23",
    scryfall_id = "af066a2d-d357-45a7-90d8-2c34b6d0ebf8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lifetap",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
