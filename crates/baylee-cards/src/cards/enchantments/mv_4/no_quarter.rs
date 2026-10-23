//! No Quarter — {3}{R} — Enchantment
//! Oracle: Whenever a creature becomes blocked by a creature with lesser power, destroy the blocking creature.
//! Oracle: Whenever a creature blocks a creature with lesser power, destroy the attacking creature.
//! Set: TMP #193 — Tempest | Scryfall ID: 7c317ee7-ed92-4e3e-92bb-502099caccf8 | Oracle ID: d238a5c1-f992-4f89-851a-de75b59f6793
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NO_QUARTER,
    oracle_id = "d238a5c1-f992-4f89-851a-de75b59f6793",
    scryfall_id = "7c317ee7-ed92-4e3e-92bb-502099caccf8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "No Quarter",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
