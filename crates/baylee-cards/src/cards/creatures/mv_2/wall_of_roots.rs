//! Wall of Roots — {1}{G} — Creature — Plant Wall
//! Oracle: Defender
//! Oracle: Put a -0/-1 counter on this creature: Add {G}. Activate only once each turn.
//! Set: TDC #278 — Tarkir: Dragonstorm Commander | Scryfall ID: e4e7d3b7-9b0c-463d-975c-ef81d7fd8dad | Oracle ID: 3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_ROOTS,
    oracle_id = "3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f",
    scryfall_id = "e4e7d3b7-9b0c-463d-975c-ef81d7fd8dad",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wall of Roots",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
