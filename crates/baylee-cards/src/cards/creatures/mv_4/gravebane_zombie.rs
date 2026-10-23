//! Gravebane Zombie — {3}{B} — Creature — Zombie
//! Oracle: If this creature would die, put it on top of its owner's library instead.
//! Set: 6ED #133 — Classic Sixth Edition | Scryfall ID: df21b384-6073-477e-bd95-fc94ca2f2f2c | Oracle ID: d04a836b-7e42-424d-9600-42a40d0dfccd
// NOT SUPPORTED: replacing Gravebane Zombie's death so it never dies or enters the graveyard; approximated with a dies trigger that puts it on top of its owner's library, so it does die: it passes through the graveyard first and "dies" triggers see it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVEBANE_ZOMBIE,
    oracle_id = "d04a836b-7e42-424d-9600-42a40d0dfccd",
    scryfall_id = "df21b384-6073-477e-bd95-fc94ca2f2f2c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "when Gravebane Zombie dies, a dies trigger puts it on top of its owner's library; it does die first (it passes through the graveyard and other creatures' \"dies\" triggers see it), because replacing its death is not in ReplacementRule",
    ),
    faces = &[face!(
        name = "Gravebane Zombie",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PutSourceOnTopOfLibrary],
    )],
);
