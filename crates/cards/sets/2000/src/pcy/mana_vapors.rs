//! Mana Vapors — {1}{U} — Sorcery
//! Oracle: Lands target player controls don't untap during their next untap step.
//! Set: PCY #38 — Prophecy | Scryfall ID: 0b6dfe49-9fd6-4fa0-b73e-e6470d8e7ca7 | Oracle ID: 8e082c95-cdc4-4f29-9915-05f83298f14d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_VAPORS,
    oracle_id = "8e082c95-cdc4-4f29-9915-05f83298f14d",
    scryfall_id = "0b6dfe49-9fd6-4fa0-b73e-e6470d8e7ca7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mana Vapors",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
