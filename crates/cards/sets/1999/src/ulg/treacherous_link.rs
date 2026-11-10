//! Treacherous Link — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: All damage that would be dealt to enchanted creature is dealt to its controller instead.
//! Set: ULG #71 — Urza's Legacy | Scryfall ID: 2e5f8581-411b-4403-9c3a-3cf2156f6779 | Oracle ID: b7902113-9ddf-4d81-a76a-b54b8a36c154
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREACHEROUS_LINK,
    oracle_id = "b7902113-9ddf-4d81-a76a-b54b8a36c154",
    scryfall_id = "2e5f8581-411b-4403-9c3a-3cf2156f6779",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Treacherous Link",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
