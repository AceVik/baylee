//! Coral Net — {U} — Enchantment — Aura
//! Oracle: Enchant green or white creature
//! Oracle: Enchanted creature has "At the beginning of your upkeep, sacrifice this creature unless you discard a card."
//! Set: TOR #35 — Torment | Scryfall ID: abe9cf1e-d398-41e0-8b11-afe1015e4fd9 | Oracle ID: 20b8911e-6701-485c-ac62-2b3cc79ec288
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CORAL_NET,
    oracle_id = "20b8911e-6701-485c-ac62-2b3cc79ec288",
    scryfall_id = "abe9cf1e-d398-41e0-8b11-afe1015e4fd9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Coral Net",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
