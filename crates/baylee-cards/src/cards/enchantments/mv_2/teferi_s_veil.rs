//! Teferi's Veil — {1}{U} — Enchantment
//! Oracle: Whenever a creature you control attacks, it phases out at end of combat. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: WTH #53 — Weatherlight | Scryfall ID: cbf39b80-d972-4f79-902f-cc613c32e446 | Oracle ID: c6db2ea4-954c-443b-9603-c41596555c31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_VEIL,
    oracle_id = "c6db2ea4-954c-443b-9603-c41596555c31",
    scryfall_id = "cbf39b80-d972-4f79-902f-cc613c32e446",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Teferi's Veil",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
