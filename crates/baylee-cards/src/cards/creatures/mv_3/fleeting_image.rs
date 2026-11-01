//! Fleeting Image — {2}{U} — Creature — Illusion
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: {1}{U}: Return this creature to its owner's hand.
//! Set: 9ED #78 — Ninth Edition | Scryfall ID: 564b8cc4-3392-4307-a5e4-5f04e52da3ab | Oracle ID: 623f3efb-5431-42d4-98aa-9caac40e4e83
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLEETING_IMAGE,
    oracle_id = "623f3efb-5431-42d4-98aa-9caac40e4e83",
    scryfall_id = "564b8cc4-3392-4307-a5e4-5f04e52da3ab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fleeting Image",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{U}"),
        &[Effect::bounce(TargetSpec::ThisObject)],
    )],
);
