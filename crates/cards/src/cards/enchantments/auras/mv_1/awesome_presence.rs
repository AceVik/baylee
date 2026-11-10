//! Awesome Presence — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked unless defending player pays {3} for each creature they control that's blocking it.
//! Set: ALL #23a — Alliances | Scryfall ID: 80017e84-87bc-4eb8-a663-5c263d8df812 | Oracle ID: d6efff61-2087-4f48-a8a8-6c465190dad9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AWESOME_PRESENCE,
    oracle_id = "d6efff61-2087-4f48-a8a8-6c465190dad9",
    scryfall_id = "80017e84-87bc-4eb8-a663-5c263d8df812",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Awesome Presence",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
