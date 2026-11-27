//! Mogg Flunkies — {1}{R} — Creature — Goblin
//! Oracle: This creature can't attack or block alone.
//! Set: J25 #580 — Foundations Jumpstart | Scryfall ID: 85041471-cfc4-4e1e-90dc-0d62d461b49b | Oracle ID: b92fc9c1-7264-4894-90b8-a193039489c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_FLUNKIES,
    oracle_id = "b92fc9c1-7264-4894-90b8-a193039489c5",
    scryfall_id = "85041471-cfc4-4e1e-90dc-0d62d461b49b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mogg Flunkies",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
