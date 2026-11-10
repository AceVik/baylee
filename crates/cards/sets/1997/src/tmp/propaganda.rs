//! Propaganda — {2}{U} — Enchantment
//! Oracle: Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: MSC #151 — Marvel Super Heroes Commander | Scryfall ID: ac943e31-26bc-4b54-b73f-460e6e402d86 | Oracle ID: ea9709b6-4c37-4d5a-b04d-cd4c42e4f9dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROPAGANDA,
    oracle_id = "ea9709b6-4c37-4d5a-b04d-cd4c42e4f9dd",
    scryfall_id = "ac943e31-26bc-4b54-b73f-460e6e402d86",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Propaganda",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
