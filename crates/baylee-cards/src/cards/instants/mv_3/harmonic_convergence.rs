//! Harmonic Convergence — {2}{G} — Instant
//! Oracle: Put all enchantments on top of their owners' libraries.
//! Set: ULG #103 — Urza's Legacy | Scryfall ID: 5aafc380-cf4d-4843-b9c3-c389d9c5e942 | Oracle ID: 400adeec-11b4-4a9c-9466-dcab524d87ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HARMONIC_CONVERGENCE,
    oracle_id = "400adeec-11b4-4a9c-9466-dcab524d87ea",
    scryfall_id = "5aafc380-cf4d-4843-b9c3-c389d9c5e942",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Harmonic Convergence",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
