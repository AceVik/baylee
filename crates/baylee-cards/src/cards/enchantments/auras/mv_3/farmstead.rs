//! Farmstead — {W}{W}{W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life."
//! Set: SUM #19 — Summer Magic / Edgar | Scryfall ID: 8cd5732c-cd54-48d8-8b32-f33782ec69da | Oracle ID: 0d71b157-09a0-4fd4-beb9-103117a784ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FARMSTEAD,
    oracle_id = "0d71b157-09a0-4fd4-beb9-103117a784ad",
    scryfall_id = "8cd5732c-cd54-48d8-8b32-f33782ec69da",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Farmstead",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
