//! Giant Turtle — {1}{G}{G} — Creature — Turtle
//! Oracle: This creature can't attack if it attacked during your last turn.
//! Set: LEG #188 — Legends | Scryfall ID: 87e5fc19-3b10-476f-9a73-e8bf4b5fbec0 | Oracle ID: 9297c0a6-1a8e-4e6e-99d6-f0877b2ec46c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_TURTLE,
    oracle_id = "9297c0a6-1a8e-4e6e-99d6-f0877b2ec46c",
    scryfall_id = "87e5fc19-3b10-476f-9a73-e8bf4b5fbec0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Giant Turtle",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TURTLE],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
        condition = Some(Condition::SourceAttackedDuringYourLastTurn),
    ),],
);
