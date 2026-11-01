//! Pretender's Claim — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature becomes blocked, tap all lands defending player controls.
//! Set: MMQ #151 — Mercadian Masques | Scryfall ID: 2cc29dab-9211-46bc-a98c-a5dbd5b0980a | Oracle ID: ac285479-46c2-49f3-a7ff-31affe23c21f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRETENDER_S_CLAIM,
    oracle_id = "ac285479-46c2-49f3-a7ff-31affe23c21f",
    scryfall_id = "2cc29dab-9211-46bc-a98c-a5dbd5b0980a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Pretender's Claim",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
