//! Rancor — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+0 and has trample.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: 2X2 #156 — Double Masters 2022 | Scryfall ID: 86d6b411-4a31-4bfc-8dd6-e19f553bb29b | Oracle ID: 9d2d6479-531c-4ce1-b52b-00e36fa63b64
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RANCOR,
    oracle_id = "9d2d6479-531c-4ce1-b52b-00e36fa63b64",
    scryfall_id = "86d6b411-4a31-4bfc-8dd6-e19f553bb29b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rancor",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
