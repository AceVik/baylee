//! Leeches — {1}{W}{W} — Sorcery
//! Oracle: Target player loses all poison counters. Leeches deals that much damage to that player.
//! Set: ME4 #18 — Masters Edition IV | Scryfall ID: a6028e2c-486b-40f8-8b74-7150eb72b9f1 | Oracle ID: 0a265cb5-47c7-4e06-9030-602e17bedae5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LEECHES,
    oracle_id = "0a265cb5-47c7-4e06-9030-602e17bedae5",
    scryfall_id = "a6028e2c-486b-40f8-8b74-7150eb72b9f1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Leeches",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
