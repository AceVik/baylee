//! Uncontrolled Infestation — {1}{R} — Enchantment — Aura
//! Oracle: Enchant nonbasic land
//! Oracle: When enchanted land becomes tapped, destroy it.
//! Set: SCG #108 — Scourge | Scryfall ID: d9ead6c3-a4e9-43e0-ae2a-6eb73033bc49 | Oracle ID: 1de91c79-e9e6-4e37-98ff-ed92fc89d4bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNCONTROLLED_INFESTATION,
    oracle_id = "1de91c79-e9e6-4e37-98ff-ed92fc89d4bc",
    scryfall_id = "d9ead6c3-a4e9-43e0-ae2a-6eb73033bc49",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Uncontrolled Infestation",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
