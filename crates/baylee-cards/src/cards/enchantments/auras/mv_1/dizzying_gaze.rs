//! Dizzying Gaze — {R} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: {R}: Enchanted creature deals 1 damage to target creature with flying.
//! Set: EXO #81 — Exodus | Scryfall ID: 71a482cf-a1cd-47b5-a76a-08e03965c679 | Oracle ID: e04f46d9-43e7-45ef-b59c-1810f30bec72
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DIZZYING_GAZE,
    oracle_id = "e04f46d9-43e7-45ef-b59c-1810f30bec72",
    scryfall_id = "71a482cf-a1cd-47b5-a76a-08e03965c679",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dizzying Gaze",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
