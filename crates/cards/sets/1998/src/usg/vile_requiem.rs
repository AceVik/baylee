//! Vile Requiem — {2}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {1}{B}, Sacrifice this enchantment: Destroy up to X target nonblack creatures, where X is the number of verse counters on this enchantment. They can't be regenerated.
//! Set: USG #167 — Urza's Saga | Scryfall ID: 2be44b37-2383-4bf4-ba30-d8d4e2cb8939 | Oracle ID: a72213e7-16f0-4fbe-91d3-750ca51a4336
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VILE_REQUIEM,
    oracle_id = "a72213e7-16f0-4fbe-91d3-750ca51a4336",
    scryfall_id = "2be44b37-2383-4bf4-ba30-d8d4e2cb8939",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Vile Requiem",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
