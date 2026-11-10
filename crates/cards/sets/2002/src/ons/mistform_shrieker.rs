//! Mistform Shrieker — {3}{U}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Oracle: Morph {3}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #96 — Onslaught | Scryfall ID: 1082eea2-5e83-48d4-b02b-a22e7cbe2054 | Oracle ID: 50fe31e9-473e-4d2f-b1b5-3eef3c7bb0e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_SHRIEKER,
    oracle_id = "50fe31e9-473e-4d2f-b1b5-3eef3c7bb0e6",
    scryfall_id = "1082eea2-5e83-48d4-b02b-a22e7cbe2054",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Shrieker",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
