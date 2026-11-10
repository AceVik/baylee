//! Wandering Stream — {2}{G} — Sorcery
//! Oracle: Domain — You gain 2 life for each basic land type among lands you control.
//! Set: INV #224 — Invasion | Scryfall ID: 6da5cb6c-253b-44f0-98f9-d75f42c6e14b | Oracle ID: 1f592b83-7838-4466-8bcb-093ac7346c55
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WANDERING_STREAM,
    oracle_id = "1f592b83-7838-4466-8bcb-093ac7346c55",
    scryfall_id = "6da5cb6c-253b-44f0-98f9-d75f42c6e14b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wandering Stream",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
