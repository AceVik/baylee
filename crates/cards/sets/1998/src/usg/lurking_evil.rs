//! Lurking Evil — {B}{B}{B} — Enchantment
//! Oracle: Pay half your life, rounded up: This enchantment becomes a 4/4 Phyrexian Horror creature with flying.
//! Set: VMA #127 — Vintage Masters | Scryfall ID: 44154580-28f8-4b2f-88b7-528010816ddf | Oracle ID: 81d6b42e-9c41-4d1e-bd28-4662e1fde572
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LURKING_EVIL,
    oracle_id = "81d6b42e-9c41-4d1e-bd28-4662e1fde572",
    scryfall_id = "44154580-28f8-4b2f-88b7-528010816ddf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Lurking Evil",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
