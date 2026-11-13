//! Thought Eater — {1}{U} — Creature — Beast
//! Oracle: Flying
//! Oracle: Your maximum hand size is reduced by three.
//! Set: ODY #106 — Odyssey | Scryfall ID: 4e05f63c-f93d-44b9-98e9-c5e3e3aad6b9 | Oracle ID: 88e18dc9-8b11-4369-9860-a687925cdcb4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHT_EATER,
    oracle_id = "88e18dc9-8b11-4369-9860-a687925cdcb4",
    scryfall_id = "4e05f63c-f93d-44b9-98e9-c5e3e3aad6b9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Eater",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
