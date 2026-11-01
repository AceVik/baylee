//! Archery Training — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, you may put an arrow counter on this Aura.
//! Oracle: Enchanted creature has "{T}: This creature deals X damage to target attacking or blocking creature, where X is the number of arrow counters on Archery Training."
//! Set: UDS #2 — Urza's Destiny | Scryfall ID: 151232e6-68cc-4cac-a532-9ade8e925961 | Oracle ID: acab4fa9-d9ab-4a89-a50f-0cee9d7ca0c3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCHERY_TRAINING,
    oracle_id = "acab4fa9-d9ab-4a89-a50f-0cee9d7ca0c3",
    scryfall_id = "151232e6-68cc-4cac-a532-9ade8e925961",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Archery Training",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
