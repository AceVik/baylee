//! Death Match — {3}{B} — Enchantment
//! Oracle: Whenever a creature enters, that creature's controller may have target creature of their choice get -3/-3 until end of turn.
//! Set: ONS #136 — Onslaught | Scryfall ID: 143e9057-267a-4c78-b72a-4f8018b627a8 | Oracle ID: 68d11f97-da97-4738-bb3e-e6730bd014e8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_MATCH,
    oracle_id = "68d11f97-da97-4738-bb3e-e6730bd014e8",
    scryfall_id = "143e9057-267a-4c78-b72a-4f8018b627a8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Death Match",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
