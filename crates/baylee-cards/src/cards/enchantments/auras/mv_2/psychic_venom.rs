//! Psychic Venom — {1}{U} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Whenever enchanted land becomes tapped, this Aura deals 2 damage to that land's controller.
//! Set: ME1 #46 — Masters Edition | Scryfall ID: 984ae881-9b43-42ce-885e-2eb5e683eed7 | Oracle ID: a60422e8-f2f4-4c37-a0f4-eedad27eb08c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PSYCHIC_VENOM,
    oracle_id = "a60422e8-f2f4-4c37-a0f4-eedad27eb08c",
    scryfall_id = "984ae881-9b43-42ce-885e-2eb5e683eed7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psychic Venom",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
