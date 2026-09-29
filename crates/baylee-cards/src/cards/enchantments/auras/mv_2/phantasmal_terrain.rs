//! Phantasmal Terrain — {U}{U} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: As this Aura enters, choose a basic land type.
//! Oracle: Enchanted land is the chosen type.
//! Set: ME4 #56 — Masters Edition IV | Scryfall ID: e9640cd5-bef8-486f-b903-46ff92efe3d3 | Oracle ID: 7dcbce46-2973-4a9f-93df-95ac41ce668a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTASMAL_TERRAIN,
    oracle_id = "7dcbce46-2973-4a9f-93df-95ac41ce668a",
    scryfall_id = "e9640cd5-bef8-486f-b903-46ff92efe3d3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Phantasmal Terrain",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
