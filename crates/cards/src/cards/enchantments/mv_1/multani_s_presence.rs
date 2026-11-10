//! Multani's Presence — {G} — Enchantment
//! Oracle: Whenever a spell you've cast is countered, draw a card.
//! Set: ULG #109 — Urza's Legacy | Scryfall ID: 38bfa984-5fe9-44ad-b13f-3276951f9f10 | Oracle ID: adad1be7-8d3d-4857-a3b0-be33db4c5746
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MULTANI_S_PRESENCE,
    oracle_id = "adad1be7-8d3d-4857-a3b0-be33db4c5746",
    scryfall_id = "38bfa984-5fe9-44ad-b13f-3276951f9f10",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Multani's Presence",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
