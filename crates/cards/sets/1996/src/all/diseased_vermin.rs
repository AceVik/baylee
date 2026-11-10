//! Diseased Vermin — {2}{B} — Creature — Rat
//! Oracle: Whenever this creature deals combat damage to a player, put an infection counter on it.
//! Oracle: At the beginning of your upkeep, this creature deals X damage to target opponent previously dealt damage by it, where X is the number of infection counters on it.
//! Set: ALL #46 — Alliances | Scryfall ID: 39703080-524d-4aa1-8c58-d512c41ae5d4 | Oracle ID: 4d18bea4-c9b2-4fc7-948f-2583589994d7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISEASED_VERMIN,
    oracle_id = "4d18bea4-c9b2-4fc7-948f-2583589994d7",
    scryfall_id = "39703080-524d-4aa1-8c58-d512c41ae5d4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Diseased Vermin",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
