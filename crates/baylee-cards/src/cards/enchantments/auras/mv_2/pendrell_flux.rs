//! Pendrell Flux — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "At the beginning of your upkeep, sacrifice this creature unless you pay its mana cost."
//! Set: USG #87 — Urza's Saga | Scryfall ID: 34847e03-c529-415e-9d49-fd4647ca8892 | Oracle ID: fe902db2-3b1d-4266-88eb-307f878fc317
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PENDRELL_FLUX,
    oracle_id = "fe902db2-3b1d-4266-88eb-307f878fc317",
    scryfall_id = "34847e03-c529-415e-9d49-fd4647ca8892",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Pendrell Flux",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
