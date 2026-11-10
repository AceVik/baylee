//! Goblin Taskmaster — {R} — Creature — Goblin
//! Oracle: {1}{R}: Target Goblin creature gets +1/+0 until end of turn.
//! Oracle: Morph {R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #210 — Onslaught | Scryfall ID: feff65ca-aedf-4434-b701-590d600d1a0b | Oracle ID: 64fddd5e-06e4-4d54-9ea6-0c9b82f3b153
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_TASKMASTER,
    oracle_id = "64fddd5e-06e4-4d54-9ea6-0c9b82f3b153",
    scryfall_id = "feff65ca-aedf-4434-b701-590d600d1a0b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Taskmaster",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
