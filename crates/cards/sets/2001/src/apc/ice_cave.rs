//! Ice Cave — {3}{U}{U} — Enchantment
//! Oracle: Whenever a player casts a spell, any other player may pay that spell's mana cost. If a player does, counter the spell. (Mana cost includes color.)
//! Set: APC #24 — Apocalypse | Scryfall ID: fc2877c2-4426-4c07-92a2-8ba5107d5e7e | Oracle ID: c532ecd2-feeb-41f5-bda0-b05e86f86a45
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ICE_CAVE,
    oracle_id = "c532ecd2-feeb-41f5-bda0-b05e86f86a45",
    scryfall_id = "fc2877c2-4426-4c07-92a2-8ba5107d5e7e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ice Cave",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
