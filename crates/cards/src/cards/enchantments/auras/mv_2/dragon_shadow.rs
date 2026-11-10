//! Dragon Shadow — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+0 and has fear. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: When a creature with mana value 6 or greater enters, you may return this card from your graveyard to the battlefield attached to that creature.
//! Set: SCG #65 — Scourge | Scryfall ID: 0ec35e03-022b-417c-9987-7379cf3956f9 | Oracle ID: c9708a12-96b5-43c1-ac2d-d6dc6b03beb7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_SHADOW,
    oracle_id = "c9708a12-96b5-43c1-ac2d-d6dc6b03beb7",
    scryfall_id = "0ec35e03-022b-417c-9987-7379cf3956f9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dragon Shadow",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
