//! Cho-Arrim Bruiser — {5}{W} — Creature — Ogre Rebel
//! Oracle: Whenever this creature attacks, you may tap up to two target creatures.
//! Set: MMQ #9 — Mercadian Masques | Scryfall ID: 26e98f06-ad8d-4a93-8ae6-3da42b63b5b5 | Oracle ID: 3db407ec-da4b-4b25-b613-e45c164ffe86
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHO_ARRIM_BRUISER,
    oracle_id = "3db407ec-da4b-4b25-b613-e45c164ffe86",
    scryfall_id = "26e98f06-ad8d-4a93-8ae6-3da42b63b5b5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cho-Arrim Bruiser",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE, subtypes::creature::REBEL],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::TapTarget],
        }],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)),
    )],
);
