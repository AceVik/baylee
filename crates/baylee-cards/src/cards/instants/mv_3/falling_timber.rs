//! Falling Timber — {2}{G} — Instant
//! Oracle: Kicker—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell.)
//! Oracle: Prevent all combat damage target creature would deal this turn. If this spell was kicked, prevent all combat damage another target creature would deal this turn.
//! Set: PLS #79 — Planeshift | Scryfall ID: 6e54c84d-ccc9-4c52-b02c-e0392e8fe447 | Oracle ID: 4ea19457-97ef-4ac8-b67c-41be1109ca73
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALLING_TIMBER,
    oracle_id = "4ea19457-97ef-4ac8-b67c-41be1109ca73",
    scryfall_id = "6e54c84d-ccc9-4c52-b02c-e0392e8fe447",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Falling Timber",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
