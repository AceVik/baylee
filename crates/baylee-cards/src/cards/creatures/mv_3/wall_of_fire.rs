//! Wall of Fire — {1}{R}{R} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: M15 #167 — Magic 2015 | Scryfall ID: 6f8ac968-3d00-40d8-80b5-e6fe08025de2 | Oracle ID: f38c8b47-e8e0-4d2f-b1da-d8d986805a48
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_FIRE,
    oracle_id = "f38c8b47-e8e0-4d2f-b1da-d8d986805a48",
    scryfall_id = "6f8ac968-3d00-40d8-80b5-e6fe08025de2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Wall of Fire",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
