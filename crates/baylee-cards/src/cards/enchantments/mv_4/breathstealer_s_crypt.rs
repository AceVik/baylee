//! Breathstealer's Crypt — {2}{U}{B} — Enchantment
//! Oracle: If a player would draw a card, instead they draw a card and reveal it. If it's a creature card, that player discards it unless they pay 3 life.
//! Set: VIS #127 — Visions | Scryfall ID: f87ace53-d77c-4df5-b200-4be2ac2b7fdb | Oracle ID: c9a9550c-e6bb-4fe3-930f-6266024dbead
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREATHSTEALER_S_CRYPT,
    oracle_id = "c9a9550c-e6bb-4fe3-930f-6266024dbead",
    scryfall_id = "f87ace53-d77c-4df5-b200-4be2ac2b7fdb",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Breathstealer's Crypt",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
