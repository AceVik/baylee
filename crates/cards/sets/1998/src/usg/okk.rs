//! Okk — {1}{R} — Creature — Goblin
//! Oracle: This creature can't attack unless a creature with greater power also attacks.
//! Oracle: This creature can't block unless a creature with greater power also blocks.
//! Set: USG #204 — Urza's Saga | Scryfall ID: 5fecbdbc-dcfb-42ee-956c-a508db6eaafa | Oracle ID: f835a8d3-e117-4669-9952-6e9ce13b35fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OKK,
    oracle_id = "f835a8d3-e117-4669-9952-6e9ce13b35fa",
    scryfall_id = "5fecbdbc-dcfb-42ee-956c-a508db6eaafa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Okk",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
