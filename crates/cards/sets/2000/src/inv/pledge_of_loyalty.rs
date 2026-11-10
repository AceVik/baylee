//! Pledge of Loyalty — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from each color among permanents you control. This effect doesn't remove this Aura.
//! Set: INV #24 — Invasion | Scryfall ID: d6f98c26-5b30-400c-8af1-8c6c43065f63 | Oracle ID: e88b3066-eb10-4ade-8207-af4de40facec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PLEDGE_OF_LOYALTY,
    oracle_id = "e88b3066-eb10-4ade-8207-af4de40facec",
    scryfall_id = "d6f98c26-5b30-400c-8af1-8c6c43065f63",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pledge of Loyalty",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
