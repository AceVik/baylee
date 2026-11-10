//! Antagonism — {3}{R} — Enchantment
//! Oracle: At the beginning of each player's end step, this enchantment deals 2 damage to that player unless one of their opponents was dealt damage this turn.
//! Set: USG #173 — Urza's Saga | Scryfall ID: 6b9db511-089e-413f-8005-ccb05ec3b06e | Oracle ID: 68f81fcf-2dcf-41b9-ad7c-588149feb12a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANTAGONISM,
    oracle_id = "68f81fcf-2dcf-41b9-ad7c-588149feb12a",
    scryfall_id = "6b9db511-089e-413f-8005-ccb05ec3b06e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Antagonism",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
