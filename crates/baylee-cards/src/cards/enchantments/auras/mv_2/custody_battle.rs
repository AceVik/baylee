//! Custody Battle — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "At the beginning of your upkeep, target opponent gains control of this creature unless you sacrifice a land."
//! Set: ONS #197 — Onslaught | Scryfall ID: b72257f5-0cf9-45ca-8dc7-a1a93bd7dd1e | Oracle ID: 9a9664d7-2299-43dc-89b6-d74fe69e0e4a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CUSTODY_BATTLE,
    oracle_id = "9a9664d7-2299-43dc-89b6-d74fe69e0e4a",
    scryfall_id = "b72257f5-0cf9-45ca-8dc7-a1a93bd7dd1e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Custody Battle",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
