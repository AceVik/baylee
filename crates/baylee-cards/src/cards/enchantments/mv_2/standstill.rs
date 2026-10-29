//! Standstill — {1}{U} — Enchantment
//! Oracle: When a player casts a spell, sacrifice this enchantment. If you do, each of that player's opponents draws three cards.
//! Set: ODY #102 — Odyssey | Scryfall ID: 3ede3f6f-e642-4fe4-aa37-0f01cdf4d149 | Oracle ID: fea4cb32-0329-4199-9abe-bff0cc45882a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STANDSTILL,
    oracle_id = "fea4cb32-0329-4199-9abe-bff0cc45882a",
    scryfall_id = "3ede3f6f-e642-4fe4-aa37-0f01cdf4d149",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Standstill",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
