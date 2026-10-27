//! Skinthinner — {1}{B} — Creature — Zombie
//! Oracle: Morph {3}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, destroy target nonblack creature. It can't be regenerated.
//! Set: C19 #129 — Commander 2019 | Scryfall ID: e0c9b43c-e544-47eb-98f5-45d1b57aabb4 | Oracle ID: c4dee5fe-36f0-4155-9437-2c0504feba2c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKINTHINNER,
    oracle_id = "c4dee5fe-36f0-4155-9437-2c0504feba2c",
    scryfall_id = "e0c9b43c-e544-47eb-98f5-45d1b57aabb4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Skinthinner",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
