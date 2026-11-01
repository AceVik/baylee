//! Mirror Strike — {3}{W} — Instant
//! Oracle: All combat damage that would be dealt to you this turn by target unblocked creature is dealt to its controller instead.
//! Set: PCY #17 — Prophecy | Scryfall ID: 148fbe36-b22d-44e6-9341-7f707baca49d | Oracle ID: 184fdd23-8235-4500-b2f5-d3b0b93a8f35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRROR_STRIKE,
    oracle_id = "184fdd23-8235-4500-b2f5-d3b0b93a8f35",
    scryfall_id = "148fbe36-b22d-44e6-9341-7f707baca49d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Mirror Strike",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
