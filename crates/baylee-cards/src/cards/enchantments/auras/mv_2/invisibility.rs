//! Invisibility — {U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked except by Walls.
//! Set: M15 #61 — Magic 2015 | Scryfall ID: 348d6493-681a-4ea9-9154-ea09f2648e8a | Oracle ID: de26b0c6-dfb7-45a8-9d7f-f8d45522d675
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INVISIBILITY,
    oracle_id = "de26b0c6-dfb7-45a8-9d7f-f8d45522d675",
    scryfall_id = "348d6493-681a-4ea9-9154-ea09f2648e8a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Invisibility",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
