//! Empyrial Armor — {1}{W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 for each card in your hand.
//! Set: VMA #25 — Vintage Masters | Scryfall ID: 6619db75-4d32-4c7b-856c-00aa33434342 | Oracle ID: ec9e8611-e785-4d5d-b6e1-bf63daf408df
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMPYRIAL_ARMOR,
    oracle_id = "ec9e8611-e785-4d5d-b6e1-bf63daf408df",
    scryfall_id = "6619db75-4d32-4c7b-856c-00aa33434342",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Empyrial Armor",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
