//! Crown of Vigor — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1.
//! Oracle: Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get +1/+1 until end of turn.
//! Set: ONS #253 — Onslaught | Scryfall ID: e7e320a6-88e2-4be1-97e2-30e0f3c2e450 | Oracle ID: 0b5f94be-7e56-44ee-9af2-d664d9f91193
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROWN_OF_VIGOR,
    oracle_id = "0b5f94be-7e56-44ee-9af2-d664d9f91193",
    scryfall_id = "e7e320a6-88e2-4be1-97e2-30e0f3c2e450",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Crown of Vigor",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
