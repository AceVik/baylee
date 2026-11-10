//! Time and Tide — {U}{U} — Instant
//! Oracle: Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out.
//! Set: VIS #46 — Visions | Scryfall ID: 152b348a-0301-4d45-a2c1-d78802c445ba | Oracle ID: b6fee94a-30b8-415e-8567-ad03ccb1d658
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_AND_TIDE,
    oracle_id = "b6fee94a-30b8-415e-8567-ad03ccb1d658",
    scryfall_id = "152b348a-0301-4d45-a2c1-d78802c445ba",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Time and Tide",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
