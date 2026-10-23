//! Omen of Fire — {3}{R}{R} — Instant
//! Oracle: Return all Islands to their owners' hands. Each player sacrifices a Plains or a white permanent of their choice for each white permanent they control.
//! Set: ALL #75 — Alliances | Scryfall ID: 9c724b46-6e17-4bee-9bc6-e9fc5a379dd7 | Oracle ID: 5935e9b3-b9b9-413c-ba17-103f5792131a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OMEN_OF_FIRE,
    oracle_id = "5935e9b3-b9b9-413c-ba17-103f5792131a",
    scryfall_id = "9c724b46-6e17-4bee-9bc6-e9fc5a379dd7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Omen of Fire",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
