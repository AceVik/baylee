//! Furnace of Rath — {1}{R}{R}{R} — Enchantment
//! Oracle: If a source would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.
//! Set: HOP #55 — Planechase | Scryfall ID: d7f0e720-3c32-4040-b663-7f99ad5bc810 | Oracle ID: 68715465-6cf9-4006-87e9-31f227fe9ed3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FURNACE_OF_RATH,
    oracle_id = "68715465-6cf9-4006-87e9-31f227fe9ed3",
    scryfall_id = "d7f0e720-3c32-4040-b663-7f99ad5bc810",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Furnace of Rath",
        mana_cost = mana!("{1}{R}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
