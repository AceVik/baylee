//! Mangara's Equity — {1}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose black or red.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {1}{W}.
//! Oracle: Whenever a creature of the chosen color deals damage to you or a white creature you control, this enchantment deals that much damage to that creature.
//! Set: MIR #26 — Mirage | Scryfall ID: 6d796153-3f7f-4d94-8ad4-5a4b2c8e09bb | Oracle ID: c7986014-30e0-4999-ab95-b86620471419
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANGARA_S_EQUITY,
    oracle_id = "c7986014-30e0-4999-ab95-b86620471419",
    scryfall_id = "6d796153-3f7f-4d94-8ad4-5a4b2c8e09bb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Mangara's Equity",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
