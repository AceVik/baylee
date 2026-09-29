//! Warp Artifact — {B}{B} — Enchantment — Aura
//! Oracle: Enchant artifact
//! Oracle: At the beginning of the upkeep of enchanted artifact's controller, this Aura deals 1 damage to that player.
//! Set: ME4 #100 — Masters Edition IV | Scryfall ID: 20f3c7e4-dfb5-410f-b017-09b8b6d76037 | Oracle ID: f4b18451-1f40-48bd-8ca9-eec784ad5dd7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARP_ARTIFACT,
    oracle_id = "f4b18451-1f40-48bd-8ca9-eec784ad5dd7",
    scryfall_id = "20f3c7e4-dfb5-410f-b017-09b8b6d76037",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Warp Artifact",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
