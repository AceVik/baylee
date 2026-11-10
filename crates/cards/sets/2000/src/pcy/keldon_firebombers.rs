//! Keldon Firebombers — {3}{R}{R} — Creature — Human Soldier
//! Oracle: When this creature enters, each player sacrifices all lands they control except for three.
//! Set: PCY #94 — Prophecy | Scryfall ID: d3fc78b5-c259-4c67-810c-99655e72c2da | Oracle ID: d5e02fa5-827e-414b-acb5-932de4bb3ee3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KELDON_FIREBOMBERS,
    oracle_id = "d5e02fa5-827e-414b-acb5-932de4bb3ee3",
    scryfall_id = "d3fc78b5-c259-4c67-810c-99655e72c2da",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Keldon Firebombers",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
