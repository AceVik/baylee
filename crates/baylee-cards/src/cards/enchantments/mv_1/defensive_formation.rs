//! Defensive Formation — {W} — Enchantment
//! Oracle: Rather than the attacking player, you assign the combat damage of each creature attacking you. You can divide that creature's combat damage as you choose among any of the creatures blocking it.
//! Set: USG #9 — Urza's Saga | Scryfall ID: fdd76db0-1a67-4965-81a5-1d8d86b63971 | Oracle ID: 9386f4d3-aa72-4849-8921-83740762dc0e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFENSIVE_FORMATION,
    oracle_id = "9386f4d3-aa72-4849-8921-83740762dc0e",
    scryfall_id = "fdd76db0-1a67-4965-81a5-1d8d86b63971",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Defensive Formation",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
