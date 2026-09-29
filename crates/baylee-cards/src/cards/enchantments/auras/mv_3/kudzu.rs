//! Kudzu — {1}{G}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: When enchanted land becomes tapped, destroy it. That land's controller may attach this Aura to a land of their choice.
//! Set: ME4 #159 — Masters Edition IV | Scryfall ID: c3bc363a-573e-4df4-9b1b-586bf4275f3c | Oracle ID: 51ca5965-ae39-4e51-8948-9a230a03f906
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KUDZU,
    oracle_id = "51ca5965-ae39-4e51-8948-9a230a03f906",
    scryfall_id = "c3bc363a-573e-4df4-9b1b-586bf4275f3c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Kudzu",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
