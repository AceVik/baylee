//! Gravel Slinger — {3}{W} — Creature — Human Soldier
//! Oracle: {T}: This creature deals 1 damage to target attacking or blocking creature.
//! Oracle: Morph {1}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #33 — Onslaught | Scryfall ID: 87551307-6b5f-4f12-aa1f-4beebefad3b3 | Oracle ID: 0aab31b7-7196-4843-b4f1-c15e9e05dd86
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVEL_SLINGER,
    oracle_id = "0aab31b7-7196-4843-b4f1-c15e9e05dd86",
    scryfall_id = "87551307-6b5f-4f12-aa1f-4beebefad3b3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Gravel Slinger",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
