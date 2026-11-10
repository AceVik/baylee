//! Latulla's Orders — {1}{R} — Enchantment — Aura
//! Oracle: Flash
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals combat damage to defending player, you may destroy target artifact that player controls.
//! Set: PCY #96 — Prophecy | Scryfall ID: a56cd728-5c3c-4fd6-bb01-0bf0875508c7 | Oracle ID: 2f3c66db-efc1-4571-a4db-371a3d532615
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LATULLA_S_ORDERS,
    oracle_id = "2f3c66db-efc1-4571-a4db-371a3d532615",
    scryfall_id = "a56cd728-5c3c-4fd6-bb01-0bf0875508c7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Latulla's Orders",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
