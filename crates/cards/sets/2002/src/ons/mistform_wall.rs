//! Mistform Wall — {2}{U} — Creature — Illusion Wall
//! Oracle: This creature has defender as long as it's a Wall.
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Set: ONS #99 — Onslaught | Scryfall ID: ebaa7a26-8516-4d71-a524-77b2d3f030d5 | Oracle ID: 02c4abd0-9a68-4a85-954b-3f64666c1f8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_WALL,
    oracle_id = "02c4abd0-9a68-4a85-954b-3f64666c1f8d",
    scryfall_id = "ebaa7a26-8516-4d71-a524-77b2d3f030d5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Wall",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION, subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
