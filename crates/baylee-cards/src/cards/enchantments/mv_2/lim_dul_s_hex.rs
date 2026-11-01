//! Lim-Dûl's Hex — {1}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}.
//! Set: ICE #146 — Ice Age | Scryfall ID: af976f42-3d56-4e32-8294-970a276a4bf3 | Oracle ID: 7f9116e1-9aab-470b-90eb-f51a3fbb3c0e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIM_DUL_S_HEX,
    oracle_id = "7f9116e1-9aab-470b-90eb-f51a3fbb3c0e",
    scryfall_id = "af976f42-3d56-4e32-8294-970a276a4bf3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Lim-Dûl's Hex",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
