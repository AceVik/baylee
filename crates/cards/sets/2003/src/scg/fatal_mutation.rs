//! Fatal Mutation — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature is turned face up, destroy it. It can't be regenerated.
//! Set: SCG #66 — Scourge | Scryfall ID: 57cf9f50-8858-44a6-8bd5-0ce1e281a584 | Oracle ID: 26d3077f-ad1f-47fb-a96d-8aac4bdef8a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FATAL_MUTATION,
    oracle_id = "26d3077f-ad1f-47fb-a96d-8aac4bdef8a3",
    scryfall_id = "57cf9f50-8858-44a6-8bd5-0ce1e281a584",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Fatal Mutation",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
