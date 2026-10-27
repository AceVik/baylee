//! Druid's Call — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature is dealt damage, its controller creates that many 1/1 green Squirrel creature tokens.
//! Set: ODY #239 — Odyssey | Scryfall ID: e22318a5-80c8-4fa3-ad05-8d3035caf336 | Oracle ID: e71f4fe8-2ecd-450b-8c41-5e38d749f59d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRUID_S_CALL,
    oracle_id = "e71f4fe8-2ecd-450b-8c41-5e38d749f59d",
    scryfall_id = "e22318a5-80c8-4fa3-ad05-8d3035caf336",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Druid's Call",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
