//! Dazzling Beauty — {2}{W} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: Target unblocked attacking creature becomes blocked. (This spell works on creatures that can't be blocked.)
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #8 — Mirage | Scryfall ID: ad0ece98-5506-4a18-b900-8d1a6cd87385 | Oracle ID: de96ef93-a2e0-409d-9d25-fcb2405039b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAZZLING_BEAUTY,
    oracle_id = "de96ef93-a2e0-409d-9d25-fcb2405039b5",
    scryfall_id = "ad0ece98-5506-4a18-b900-8d1a6cd87385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dazzling Beauty",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
