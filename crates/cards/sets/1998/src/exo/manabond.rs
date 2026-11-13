//! Manabond — {G} — Enchantment
//! Oracle: At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.
//! Set: TPR #179 — Tempest Remastered | Scryfall ID: 6f94ab94-5a8f-4422-8927-0b7da85c3119 | Oracle ID: 482006a0-bb2a-4533-891b-2195da5e0f01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANABOND,
    oracle_id = "482006a0-bb2a-4533-891b-2195da5e0f01",
    scryfall_id = "6f94ab94-5a8f-4422-8927-0b7da85c3119",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Manabond",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
