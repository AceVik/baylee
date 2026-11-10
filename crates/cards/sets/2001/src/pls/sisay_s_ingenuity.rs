//! Sisay's Ingenuity — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, draw a card.
//! Oracle: Enchanted creature has "{2}{U}: Target creature becomes the color of your choice until end of turn."
//! Set: PLS #33 — Planeshift | Scryfall ID: bbe20cc1-621a-4813-9bbb-ace006e173ff | Oracle ID: 3a487f20-6b1d-4be3-b51f-0c8f0a611601
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SISAY_S_INGENUITY,
    oracle_id = "3a487f20-6b1d-4be3-b51f-0c8f0a611601",
    scryfall_id = "bbe20cc1-621a-4813-9bbb-ace006e173ff",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sisay's Ingenuity",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
