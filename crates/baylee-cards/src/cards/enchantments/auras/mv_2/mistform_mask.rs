//! Mistform Mask — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {1}: Enchanted creature becomes the creature type of your choice until end of turn.
//! Set: ONS #94 — Onslaught | Scryfall ID: 7fbbb075-5795-425f-9e33-70cb922eea16 | Oracle ID: bbb2ac2e-de5c-441a-9707-5ced64df43a6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_MASK,
    oracle_id = "bbb2ac2e-de5c-441a-9707-5ced64df43a6",
    scryfall_id = "7fbbb075-5795-425f-9e33-70cb922eea16",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Mask",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
