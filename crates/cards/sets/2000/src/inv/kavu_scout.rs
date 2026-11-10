//! Kavu Scout — {2}{R} — Creature — Kavu Scout
//! Oracle: Domain — This creature gets +1/+0 for each basic land type among lands you control.
//! Set: INV #151 — Invasion | Scryfall ID: cbc2670d-a3f4-47c2-b424-01fd379ff186 | Oracle ID: 2bd91ce8-1797-4dd7-8150-0d4caa7dad61
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_SCOUT,
    oracle_id = "2bd91ce8-1797-4dd7-8150-0d4caa7dad61",
    scryfall_id = "cbc2670d-a3f4-47c2-b424-01fd379ff186",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kavu Scout",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU, subtypes::creature::SCOUT],
        power = Some(0),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
