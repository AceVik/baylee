//! Red Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from red. This effect doesn't remove this Aura.
//! Set: 4ED #44 — Fourth Edition | Scryfall ID: 543e8ec0-f36a-46a6-b7b1-e983e90a091a | Oracle ID: 73f84440-425f-4cf5-b01a-3ae89f1f6e37
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RED_WARD,
    oracle_id = "73f84440-425f-4cf5-b01a-3ae89f1f6e37",
    scryfall_id = "543e8ec0-f36a-46a6-b7b1-e983e90a091a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Red Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
