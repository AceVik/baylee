//! Blessing — {W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {W}: Enchanted creature gets +1/+1 until end of turn.
//! Set: M14 #8 — Magic 2014 | Scryfall ID: 4273cf50-db65-4b1f-95e1-f24ba6582c8b | Oracle ID: 5c84d8da-2bfb-4618-89a0-7d9ed604e854
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLESSING,
    oracle_id = "5c84d8da-2bfb-4618-89a0-7d9ed604e854",
    scryfall_id = "4273cf50-db65-4b1f-95e1-f24ba6582c8b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Blessing",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
