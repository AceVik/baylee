//! Melee — {4}{R} — Instant
//! Oracle: Cast this spell only during combat on your turn before blockers are declared.
//! Oracle: You choose which creatures block this combat and how those creatures block.
//! Oracle: Whenever a creature attacks and isn't blocked this combat, untap it and remove it from combat.
//! Set: ICE #200 — Ice Age | Scryfall ID: b13a064d-bff4-4a48-a158-1b61951b0ac3 | Oracle ID: 93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MELEE,
    oracle_id = "93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e",
    scryfall_id = "b13a064d-bff4-4a48-a158-1b61951b0ac3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Melee",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
