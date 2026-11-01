//! Squandered Resources — {B}{G} — Enchantment
//! Oracle: Sacrifice a land: Add one mana of any type the sacrificed land could produce.
//! Set: VIS #137 — Visions | Scryfall ID: fcddbea7-3025-47b1-a597-2d2b2711fb81 | Oracle ID: 620d0572-6d22-43e2-83a9-000e605c8a2b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SQUANDERED_RESOURCES,
    oracle_id = "620d0572-6d22-43e2-83a9-000e605c8a2b",
    scryfall_id = "fcddbea7-3025-47b1-a597-2d2b2711fb81",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Squandered Resources",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
