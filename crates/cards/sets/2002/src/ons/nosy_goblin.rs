//! Nosy Goblin — {2}{R} — Creature — Goblin
//! Oracle: {T}, Sacrifice this creature: Destroy target face-down creature.
//! Set: ONS #220 — Onslaught | Scryfall ID: 70ea023e-e66d-4049-b7bc-5e660804f088 | Oracle ID: 7bca544e-08d8-4e83-aa30-c7201a649a66
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOSY_GOBLIN,
    oracle_id = "7bca544e-08d8-4e83-aa30-c7201a649a66",
    scryfall_id = "70ea023e-e66d-4049-b7bc-5e660804f088",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Nosy Goblin",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
