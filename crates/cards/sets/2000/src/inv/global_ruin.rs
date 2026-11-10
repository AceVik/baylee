//! Global Ruin — {4}{W} — Sorcery
//! Oracle: Each player chooses from the lands they control a land of each basic land type, then sacrifices the rest.
//! Set: INV #18 — Invasion | Scryfall ID: 336474b4-2cf5-44c0-b72c-f75f1a7ed928 | Oracle ID: 4a7b89b2-5fe9-4d39-9d42-3ca78bcf6d40
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLOBAL_RUIN,
    oracle_id = "4a7b89b2-5fe9-4d39-9d42-3ca78bcf6d40",
    scryfall_id = "336474b4-2cf5-44c0-b72c-f75f1a7ed928",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Global Ruin",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
