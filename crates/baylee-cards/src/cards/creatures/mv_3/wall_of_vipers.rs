//! Wall of Vipers — {2}{B} — Creature — Snake Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {3}: Destroy this creature and target creature it's blocking. Any player may activate this ability.
//! Set: PCY #80 — Prophecy | Scryfall ID: 00042443-4d4e-4087-b4e5-5e781e7cc5fa | Oracle ID: 4fcfe370-ba33-438f-bd96-2ae560f59df9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_VIPERS,
    oracle_id = "4fcfe370-ba33-438f-bd96-2ae560f59df9",
    scryfall_id = "00042443-4d4e-4087-b4e5-5e781e7cc5fa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wall of Vipers",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE, subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
