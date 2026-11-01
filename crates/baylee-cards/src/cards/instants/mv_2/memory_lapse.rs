//! Memory Lapse — {1}{U} — Instant
//! Oracle: Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard.
//! Set: EMA #60 — Eternal Masters | Scryfall ID: 30202613-d05f-4f47-af97-d0b75ccac293 | Oracle ID: bbfb3e4a-b389-4391-8141-13b68c0ef2e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORY_LAPSE,
    oracle_id = "bbfb3e4a-b389-4391-8141-13b68c0ef2e0",
    scryfall_id = "30202613-d05f-4f47-af97-d0b75ccac293",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Memory Lapse",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
