//! Scorching Lava — {1}{R} — Instant
//! Oracle: Kicker {R} (You may pay an additional {R} as you cast this spell.)
//! Oracle: Scorching Lava deals 2 damage to any target. If this spell was kicked, that creature can't be regenerated this turn and if it would die this turn, exile it instead.
//! Set: INV #164 — Invasion | Scryfall ID: 2a85437f-052e-494c-a9ee-265c4624a409 | Oracle ID: e42fb51e-254a-43ac-ad02-9f0fad0f4c8a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCORCHING_LAVA,
    oracle_id = "e42fb51e-254a-43ac-ad02-9f0fad0f4c8a",
    scryfall_id = "2a85437f-052e-494c-a9ee-265c4624a409",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scorching Lava",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
