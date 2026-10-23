//! Mogg Flunkies — {1}{R} — Creature — Goblin
//! Oracle: This creature can't attack or block alone.
//! Set: A25 #143 — Masters 25 | Scryfall ID: 5507a123-cf1a-42e1-a8fa-221960c48c16 | Oracle ID: b92fc9c1-7264-4894-90b8-a193039489c5
// PARTIAL: no modifier restricts attacking or blocking alone, so it attacks and blocks alone as a 3/3

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_FLUNKIES,
    oracle_id = "b92fc9c1-7264-4894-90b8-a193039489c5",
    scryfall_id = "5507a123-cf1a-42e1-a8fa-221960c48c16",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no modifier restricts attacking or blocking alone, so it attacks and blocks alone as a 3/3"
    ),
    faces = &[face!(
        name = "Mogg Flunkies",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(3),
    ),],
);
