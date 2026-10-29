//! Orim's Touch — {W} — Instant
//! Oracle: Kicker {1} (You may pay an additional {1} as you cast this spell.)
//! Oracle: Prevent the next 2 damage that would be dealt to any target this turn. If this spell was kicked, prevent the next 4 damage that would be dealt to that permanent or player this turn instead.
//! Set: INV #23 — Invasion | Scryfall ID: 559f551e-7891-4c6d-8798-a25c0255fa3b | Oracle ID: cc22c210-efd0-494c-8560-448b038b3c5f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORIM_S_TOUCH,
    oracle_id = "cc22c210-efd0-494c-8560-448b038b3c5f",
    scryfall_id = "559f551e-7891-4c6d-8798-a25c0255fa3b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Orim's Touch",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
