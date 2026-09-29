//! Holy Armor — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +0/+2.
//! Oracle: {W}: Enchanted creature gets +0/+1 until end of turn.
//! Set: 4ED #29 — Fourth Edition | Scryfall ID: 3a0b9dba-f621-4700-8a36-bf326e04cbac | Oracle ID: 912164c2-b4d4-42e3-a10e-903b8c7b2e6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HOLY_ARMOR,
    oracle_id = "912164c2-b4d4-42e3-a10e-903b8c7b2e6d",
    scryfall_id = "3a0b9dba-f621-4700-8a36-bf326e04cbac",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Holy Armor",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
