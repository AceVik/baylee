//! Caustic Tar — {4}{B}{B} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Target player loses 3 life."
//! Set: A25 #81 — Masters 25 | Scryfall ID: 9002443d-a296-4f68-8a94-991165e6ea38 | Oracle ID: 2b5f70ec-33da-4dff-9cff-fe05eb906685
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CAUSTIC_TAR,
    oracle_id = "2b5f70ec-33da-4dff-9cff-fe05eb906685",
    scryfall_id = "9002443d-a296-4f68-8a94-991165e6ea38",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Caustic Tar",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
