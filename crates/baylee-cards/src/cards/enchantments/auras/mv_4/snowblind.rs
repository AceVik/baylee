//! Snowblind — {3}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -X/-Y. If that creature is attacking, X is the number of snow lands defending player controls. Otherwise, X is the number of snow lands its controller controls. Y is equal to X or to enchanted creature's toughness minus 1, whichever is smaller.
//! Set: ICE #264 — Ice Age | Scryfall ID: 5f62c376-487a-42bc-bd85-ab8b0480f7dc | Oracle ID: 3f8bfa3d-7490-4259-a403-d13585ed2f66
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SNOWBLIND,
    oracle_id = "3f8bfa3d-7490-4259-a403-d13585ed2f66",
    scryfall_id = "5f62c376-487a-42bc-bd85-ab8b0480f7dc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Snowblind",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
