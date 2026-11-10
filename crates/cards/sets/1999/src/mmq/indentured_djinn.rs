//! Indentured Djinn — {1}{U}{U} — Creature — Djinn
//! Oracle: Flying
//! Oracle: When this creature enters, each other player may draw up to three cards.
//! Set: MMQ #85 — Mercadian Masques | Scryfall ID: dae62ce7-852b-42c6-9cbe-4807d8bf5740 | Oracle ID: 8ccb3bca-73da-450d-a9d0-61d3e2bd05a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INDENTURED_DJINN,
    oracle_id = "8ccb3bca-73da-450d-a9d0-61d3e2bd05a5",
    scryfall_id = "dae62ce7-852b-42c6-9cbe-4807d8bf5740",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Indentured Djinn",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
