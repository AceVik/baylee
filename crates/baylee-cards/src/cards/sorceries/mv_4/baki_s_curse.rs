//! Baki's Curse — {2}{U}{U} — Sorcery
//! Oracle: Baki's Curse deals 2 damage to each creature for each Aura attached to that creature.
//! Set: HML #22 — Homelands | Scryfall ID: e3261b4c-7963-4ca0-875d-77b7c8571b3f | Oracle ID: 341a92b9-d6a0-499e-abb7-e2b7d7bc248c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BAKI_S_CURSE,
    oracle_id = "341a92b9-d6a0-499e-abb7-e2b7d7bc248c",
    scryfall_id = "e3261b4c-7963-4ca0-875d-77b7c8571b3f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Baki's Curse",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
