//! Steam Vines — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: When enchanted land becomes tapped, destroy it and this Aura deals 1 damage to that land's controller. That player attaches this Aura to a land of their choice.
//! Set: ODY #223 — Odyssey | Scryfall ID: d3936053-b2ed-49b8-ab69-fe47a972cfe4 | Oracle ID: 1e687c04-b573-425d-b2d1-0e0cbe02bb52
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STEAM_VINES,
    oracle_id = "1e687c04-b573-425d-b2d1-0e0cbe02bb52",
    scryfall_id = "d3936053-b2ed-49b8-ab69-fe47a972cfe4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Steam Vines",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
