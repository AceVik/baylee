//! Crown of Suspicion — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/-1.
//! Oracle: Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get +2/-1 until end of turn.
//! Set: ONS #134 — Onslaught | Scryfall ID: 8953e11b-cc3a-4c8d-9d7e-04bf90c77027 | Oracle ID: 6586f8d1-196d-42a4-816f-629d432ece2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROWN_OF_SUSPICION,
    oracle_id = "6586f8d1-196d-42a4-816f-629d432ece2f",
    scryfall_id = "8953e11b-cc3a-4c8d-9d7e-04bf90c77027",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Crown of Suspicion",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
