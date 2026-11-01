//! Bane of the Living — {2}{B}{B} — Creature — Insect
//! Oracle: Morph {X}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, all creatures get -X/-X until end of turn.
//! Set: C19 #104 — Commander 2019 | Scryfall ID: 946a0fb7-0e36-47d5-8083-09f48725dff1 | Oracle ID: c99e4768-192c-49bf-8f51-c204a2248326
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BANE_OF_THE_LIVING,
    oracle_id = "c99e4768-192c-49bf-8f51-c204a2248326",
    scryfall_id = "946a0fb7-0e36-47d5-8083-09f48725dff1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bane of the Living",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
