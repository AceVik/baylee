//! Strength of Unity — {3}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Domain — Enchanted creature gets +1/+1 for each basic land type among lands you control.
//! Set: INV #40 — Invasion | Scryfall ID: 1a9d4ff8-af35-413f-9aa2-f4c6e34fade2 | Oracle ID: 00606d5e-a134-4363-9ee6-05e03d58f234
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRENGTH_OF_UNITY,
    oracle_id = "00606d5e-a134-4363-9ee6-05e03d58f234",
    scryfall_id = "1a9d4ff8-af35-413f-9aa2-f4c6e34fade2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Strength of Unity",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
