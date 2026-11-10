//! Hidden Herd — {G} — Enchantment
//! Oracle: When an opponent plays a nonbasic land, if this permanent is an enchantment, it becomes a 3/3 Beast creature.
//! Set: USG #262 — Urza's Saga | Scryfall ID: 20f07992-1f81-440a-a80a-164c44d0a471 | Oracle ID: 8e7c5649-41c2-4a0c-b873-e6f6c5908567
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_HERD,
    oracle_id = "8e7c5649-41c2-4a0c-b873-e6f6c5908567",
    scryfall_id = "20f07992-1f81-440a-a80a-164c44d0a471",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hidden Herd",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
