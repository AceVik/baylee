//! Prismatic Boon — {X}{W}{U} — Instant
//! Oracle: Choose a color. X target creatures gain protection from the chosen color until end of turn.
//! Set: MIR #274 — Mirage | Scryfall ID: fb774a9b-d29f-4f41-9fb8-a0189205e16f | Oracle ID: aee5dda1-2181-40d5-9846-85d154e1d5da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_BOON,
    oracle_id = "aee5dda1-2181-40d5-9846-85d154e1d5da",
    scryfall_id = "fb774a9b-d29f-4f41-9fb8-a0189205e16f",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Prismatic Boon",
        mana_cost = mana!("{X}{W}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
