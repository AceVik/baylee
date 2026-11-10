//! Celestial Dawn — {1}{W}{W} — Enchantment
//! Oracle: Lands you control are Plains.
//! Oracle: Nonland permanents you control are white. The same is true for spells you control and nonland cards you own that aren't on the battlefield.
//! Oracle: You may spend white mana as though it were mana of any color. You may spend other mana only as though it were colorless mana.
//! Set: TSB #3 — Time Spiral Timeshifted | Scryfall ID: 251880e9-a472-4f33-9988-4871d31aa1d9 | Oracle ID: b4d27fad-9a94-4760-80a6-6a2948109864
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CELESTIAL_DAWN,
    oracle_id = "b4d27fad-9a94-4760-80a6-6a2948109864",
    scryfall_id = "251880e9-a472-4f33-9988-4871d31aa1d9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Celestial Dawn",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
