//! Hexdrinker — {G} — Creature — Snake
//! Oracle: Level up {1} ({1}: Put a level counter on this. Level up only as a sorcery.)
//! Oracle: LEVEL 3-7
//! Oracle: 4/4
//! Oracle: Protection from instants
//! Oracle: LEVEL 8+
//! Oracle: 6/6
//! Oracle: Protection from everything
//! Set: MH1 #168 — Modern Horizons | Scryfall ID: 89f5cc05-5d9d-4709-b3c5-a6249c294acc | Oracle ID: 69bc2afd-9f53-47f2-b9c8-f12732784e10
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEXDRINKER,
    oracle_id = "69bc2afd-9f53-47f2-b9c8-f12732784e10",
    scryfall_id = "89f5cc05-5d9d-4709-b3c5-a6249c294acc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hexdrinker",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
