//! Wall of Water — {1}{U}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {U}: This creature gets +1/+0 until end of turn.
//! Set: 4ED #114 — Fourth Edition | Scryfall ID: 78028eda-61b0-408c-b3fc-adc968d39b47 | Oracle ID: 608cc65c-f99a-4ca3-be24-190d2556b411
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_WATER,
    oracle_id = "608cc65c-f99a-4ca3-be24-190d2556b411",
    scryfall_id = "78028eda-61b0-408c-b3fc-adc968d39b47",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wall of Water",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
