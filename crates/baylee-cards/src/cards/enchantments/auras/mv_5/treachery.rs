//! Treachery — {3}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, untap up to five lands.
//! Oracle: You control enchanted creature.
//! Set: UDS #50 — Urza's Destiny | Scryfall ID: 613694aa-b169-400d-8063-2b83d8303611 | Oracle ID: 8ed57194-7508-4aef-9373-64f7e80612d8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREACHERY,
    oracle_id = "8ed57194-7508-4aef-9373-64f7e80612d8",
    scryfall_id = "613694aa-b169-400d-8063-2b83d8303611",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Treachery",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
