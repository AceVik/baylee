//! Timid Drake — {2}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When another creature enters, return this creature to its owner's hand.
//! Set: MMQ #111 — Mercadian Masques | Scryfall ID: 9212f685-d7af-4279-b17e-7201d8f63813 | Oracle ID: aa8c5467-190a-4c6a-872c-56b8f8d01a2b
// IMPLEMENTED — Flying; returns to its owner's hand when another creature enters.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIMID_DRAKE,
    oracle_id = "aa8c5467-190a-4c6a-872c-56b8f8d01a2b",
    scryfall_id = "9212f685-d7af-4279-b17e-7201d8f63813",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Timid Drake",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::ANOTHER_CREATURE),
        &[Effect::bounce(TargetSpec::ThisObject)]
    ),],
);
