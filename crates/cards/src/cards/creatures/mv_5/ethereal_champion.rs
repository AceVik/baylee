//! Ethereal Champion — {2}{W}{W}{W} — Creature — Avatar
//! Oracle: Pay 1 life: Prevent the next 1 damage that would be dealt to this creature this turn.
//! Set: 6ED #20 — Classic Sixth Edition | Scryfall ID: 7c1733e2-bee2-4b85-b165-d4329402578b | Oracle ID: f203671b-76f7-4be0-80a9-826fbb450fe7
// IMPLEMENTED — Pay 1 life: prevent the next 1 damage that would be dealt to Ethereal Champion this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ETHEREAL_CHAMPION,
    oracle_id = "f203671b-76f7-4be0-80a9-826fbb450fe7",
    scryfall_id = "7c1733e2-bee2-4b85-b165-d4329402578b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ethereal Champion",
        mana_cost = mana!("{2}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(PayLife(1)),
        &[Effect::PreventNextDamage {
            target: TargetSpec::ThisObject,
            amount: Amount::Fixed(1),
        }],
    )],
);
