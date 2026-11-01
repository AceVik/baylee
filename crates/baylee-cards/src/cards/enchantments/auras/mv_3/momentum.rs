//! Momentum — {2}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, you may put a growth counter on this Aura.
//! Oracle: Enchanted creature gets +1/+1 for each growth counter on this Aura.
//! Set: UDS #113 — Urza's Destiny | Scryfall ID: 10bd11a2-7cab-4d3f-b52b-f5bb66fbbec6 | Oracle ID: 80875914-df4c-4b05-8c4d-658ff477c956
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOMENTUM,
    oracle_id = "80875914-df4c-4b05-8c4d-658ff477c956",
    scryfall_id = "10bd11a2-7cab-4d3f-b52b-f5bb66fbbec6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Momentum",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
