//! Theorist's Proxy — {1}{U} — Creature — Illusion
//! Oracle: Flash
//! Oracle: When this creature enters, empower Jace 3. (Put three loyalty counters on a Jace token you control. If you don't control one, first create a blue Jace planeswalker token with "[−1]: Surveil 1" and "[−3]: Draw a card.")
//! Oracle: {U}, Sacrifice this creature: The next spell you cast this turn can't be countered.
//! Set: FRA #44 — Reality Fracture | Scryfall ID: 710302ca-c4be-4069-8ce1-f531414c74e9 | Oracle ID: 0089acfe-da66-4dd7-b1e5-4d7407f58257
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THEORIST_S_PROXY,
    oracle_id = "0089acfe-da66-4dd7-b1e5-4d7407f58257",
    scryfall_id = "710302ca-c4be-4069-8ce1-f531414c74e9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Theorist's Proxy",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(0),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
