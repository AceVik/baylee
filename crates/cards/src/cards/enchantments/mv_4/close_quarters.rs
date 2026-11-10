//! Close Quarters — {2}{R}{R} — Enchantment
//! Oracle: Whenever a creature you control becomes blocked, this enchantment deals 1 damage to any target.
//! Set: MMQ #184 — Mercadian Masques | Scryfall ID: 1b9131c7-4e46-4c01-80b3-a6b055439346 | Oracle ID: c3f841c4-6e12-46fc-802a-3025182e7ff4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLOSE_QUARTERS,
    oracle_id = "c3f841c4-6e12-46fc-802a-3025182e7ff4",
    scryfall_id = "1b9131c7-4e46-4c01-80b3-a6b055439346",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Close Quarters",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
