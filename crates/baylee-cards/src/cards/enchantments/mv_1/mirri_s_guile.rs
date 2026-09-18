//! Mirri's Guile — {G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may look at the top three cards of your library, then put them back in any order.
//! Set: TMP #236 — Tempest | Scryfall ID: 73d51a3c-95c0-4810-b847-4b8afd12fd64 | Oracle ID: 7f89c0ee-b914-406c-8a3f-98424a52ae14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRRI_S_GUILE,
    oracle_id = "7f89c0ee-b914-406c-8a3f-98424a52ae14",
    scryfall_id = "73d51a3c-95c0-4810-b847-4b8afd12fd64",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Mirri's Guile",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
