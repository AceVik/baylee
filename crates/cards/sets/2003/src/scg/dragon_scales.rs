//! Dragon Scales — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+2 and has vigilance.
//! Oracle: When a creature with mana value 6 or greater enters, you may return this card from your graveyard to the battlefield attached to that creature.
//! Set: SCG #10 — Scourge | Scryfall ID: 8e78b364-015d-4074-ad9e-55c973ce2f4b | Oracle ID: 04628d92-bd5e-4aff-830d-7b302bd6459e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_SCALES,
    oracle_id = "04628d92-bd5e-4aff-830d-7b302bd6459e",
    scryfall_id = "8e78b364-015d-4074-ad9e-55c973ce2f4b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dragon Scales",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
