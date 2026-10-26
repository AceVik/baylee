//! Steal Strength — {1}{B} — Instant
//! Oracle: Target creature gets +1/+1 until end of turn. Another target creature gets -1/-1 until end of turn.
//! Set: PCY #79 — Prophecy | Scryfall ID: 5470b3bb-5061-4beb-9f44-b56c3b2fd816 | Oracle ID: e67434e6-5408-416a-be19-64291fca8e3e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STEAL_STRENGTH,
    oracle_id = "e67434e6-5408-416a-be19-64291fca8e3e",
    scryfall_id = "5470b3bb-5061-4beb-9f44-b56c3b2fd816",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Steal Strength",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
