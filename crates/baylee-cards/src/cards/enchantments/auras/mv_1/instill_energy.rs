//! Instill Energy — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can attack as though it had haste.
//! Oracle: {0}: Untap enchanted creature. Activate only during your turn and only once each turn.
//! Set: ME4 #157 — Masters Edition IV | Scryfall ID: a7d5d27f-759f-4e99-8ac4-e4b40865d557 | Oracle ID: 8695c3c1-fb4b-4429-ae33-ec186f68796b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INSTILL_ENERGY,
    oracle_id = "8695c3c1-fb4b-4429-ae33-ec186f68796b",
    scryfall_id = "a7d5d27f-759f-4e99-8ac4-e4b40865d557",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Instill Energy",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
