//! Fallen Cleric — {4}{B} — Creature — Zombie Cleric
//! Oracle: Protection from Clerics
//! Oracle: Morph {4}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #145 — Onslaught | Scryfall ID: 7652dc61-9170-4895-a0bf-c32a1ee0350e | Oracle ID: 7330201a-41ff-44bf-a623-2ed062878c58
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FALLEN_CLERIC,
    oracle_id = "7330201a-41ff-44bf-a623-2ed062878c58",
    scryfall_id = "7652dc61-9170-4895-a0bf-c32a1ee0350e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Fallen Cleric",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::CLERIC],
        power = Some(4),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
