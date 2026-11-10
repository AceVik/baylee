//! Sunken Field — {1}{U} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Counter target spell unless its controller pays {1}."
//! Set: PCY #51 — Prophecy | Scryfall ID: 9211ee02-e854-4414-92b1-65a7af29f0b9 | Oracle ID: 46569712-1f28-41cb-81be-fd938408c71f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUNKEN_FIELD,
    oracle_id = "46569712-1f28-41cb-81be-fd938408c71f",
    scryfall_id = "9211ee02-e854-4414-92b1-65a7af29f0b9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sunken Field",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
