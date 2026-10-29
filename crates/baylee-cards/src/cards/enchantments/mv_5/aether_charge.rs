//! Aether Charge — {4}{R} — Enchantment
//! Oracle: Whenever a Beast you control enters, you may have it deal 4 damage to target opponent or planeswalker.
//! Set: ONS #184 — Onslaught | Scryfall ID: 05df2792-4971-49e8-a8f2-17700e247500 | Oracle ID: 15e83068-6253-4c65-8679-7295f3dc2075
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_CHARGE,
    oracle_id = "15e83068-6253-4c65-8679-7295f3dc2075",
    scryfall_id = "05df2792-4971-49e8-a8f2-17700e247500",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Aether Charge",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
