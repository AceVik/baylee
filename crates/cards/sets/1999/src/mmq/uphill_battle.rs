//! Uphill Battle — {2}{R} — Enchantment
//! Oracle: Creatures played by your opponents enter tapped.
//! Set: MMQ #222 — Mercadian Masques | Scryfall ID: 73fa3455-3ba0-41ad-aefd-40f183aed2a6 | Oracle ID: 349a18bf-5c61-4fe6-a5e5-5e7cad6b4ba6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UPHILL_BATTLE,
    oracle_id = "349a18bf-5c61-4fe6-a5e5-5e7cad6b4ba6",
    scryfall_id = "73fa3455-3ba0-41ad-aefd-40f183aed2a6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Uphill Battle",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
