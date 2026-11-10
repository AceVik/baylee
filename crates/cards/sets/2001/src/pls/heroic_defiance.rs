//! Heroic Defiance — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+3 unless it shares a color with the most common color among all permanents or a color tied for most common.
//! Set: PLS #6 — Planeshift | Scryfall ID: 0dc1aa36-5d3b-4d25-9d54-937cdabf72a4 | Oracle ID: f6209f18-9f40-44b4-9a8c-3445eda89928
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEROIC_DEFIANCE,
    oracle_id = "f6209f18-9f40-44b4-9a8c-3445eda89928",
    scryfall_id = "0dc1aa36-5d3b-4d25-9d54-937cdabf72a4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Heroic Defiance",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
