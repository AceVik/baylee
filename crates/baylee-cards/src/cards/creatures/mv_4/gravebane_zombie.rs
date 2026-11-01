//! Gravebane Zombie — {3}{B} — Creature — Zombie
//! Oracle: If this creature would die, put it on top of its owner's library instead.
//! Set: 6ED #133 — Classic Sixth Edition | Scryfall ID: df21b384-6073-477e-bd95-fc94ca2f2f2c | Oracle ID: d04a836b-7e42-424d-9600-42a40d0dfccd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVEBANE_ZOMBIE,
    oracle_id = "d04a836b-7e42-424d-9600-42a40d0dfccd",
    scryfall_id = "df21b384-6073-477e-bd95-fc94ca2f2f2c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gravebane Zombie",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
