//! Sneak Attack — {3}{R} — Enchantment
//! Oracle: {R}: You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.
//! Set: USG #218 — Urza's Saga | Scryfall ID: d07dc95d-82a8-4a58-8ea2-d4513bd7316d | Oracle ID: e7aee670-e6ac-4b3a-b3d1-7b7013db8f3d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNEAK_ATTACK,
    oracle_id = "e7aee670-e6ac-4b3a-b3d1-7b7013db8f3d",
    scryfall_id = "d07dc95d-82a8-4a58-8ea2-d4513bd7316d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Sneak Attack",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
