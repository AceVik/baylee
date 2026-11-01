//! Kyren Legate — {1}{R} — Creature — Goblin
//! Oracle: If an opponent controls a Plains and you control a Mountain, you may cast this spell without paying its mana cost.
//! Oracle: Haste
//! Set: MMQ #197 — Mercadian Masques | Scryfall ID: 6f0e9806-be8c-4b88-a4be-0111d1be81d9 | Oracle ID: 41b46aa5-6091-4bc4-8d17-d2c6b5fe1c5e
// NOT SUPPORTED: conditional free cast (opponent controls Plains and you control Mountain) not supported
// PARTIAL: conditional free cast (opponent controls Plains and you control Mountain) not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KYREN_LEGATE,
    oracle_id = "41b46aa5-6091-4bc4-8d17-d2c6b5fe1c5e",
    scryfall_id = "6f0e9806-be8c-4b88-a4be-0111d1be81d9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Partial(
        "conditional free cast (opponent controls Plains and you control Mountain) not supported"
    ),
    faces = &[face!(
        name = "Kyren Legate",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
);
