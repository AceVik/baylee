//! Reins of Power — {2}{U}{U} — Instant
//! Oracle: Untap all creatures you control and all creatures target opponent controls. You and that opponent each gain control of all creatures the other controls until end of turn. Those creatures gain haste until end of turn.
//! Set: CLB #734 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 9b5730aa-c74a-4c74-bb1b-a133a3c353ec | Oracle ID: 96381388-1e82-411c-8290-1f3e909d3b5f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REINS_OF_POWER,
    oracle_id = "96381388-1e82-411c-8290-1f3e909d3b5f",
    scryfall_id = "9b5730aa-c74a-4c74-bb1b-a133a3c353ec",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Reins of Power",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
