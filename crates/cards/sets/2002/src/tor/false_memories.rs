//! False Memories — {1}{U} — Instant
//! Oracle: Mill seven cards. At the beginning of the next end step, exile seven cards from your graveyard.
//! Set: TOR #37 — Torment | Scryfall ID: 3a45ea90-03af-446e-9df3-ef64e9613f2c | Oracle ID: d95dd8cc-ce7d-4d43-afbd-efaa2d3c7a35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALSE_MEMORIES,
    oracle_id = "d95dd8cc-ce7d-4d43-afbd-efaa2d3c7a35",
    scryfall_id = "3a45ea90-03af-446e-9df3-ef64e9613f2c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "False Memories",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
