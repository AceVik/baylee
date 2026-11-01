//! Spinal Graft — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+3.
//! Oracle: When enchanted creature becomes the target of a spell or ability, destroy that creature. It can't be regenerated.
//! Set: TPR #119 — Tempest Remastered | Scryfall ID: ace63a68-82cc-47fb-b403-7deaeabd6b3e | Oracle ID: 70b5eca0-c61b-4066-ae5f-90948912c04e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINAL_GRAFT,
    oracle_id = "70b5eca0-c61b-4066-ae5f-90948912c04e",
    scryfall_id = "ace63a68-82cc-47fb-b403-7deaeabd6b3e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spinal Graft",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
