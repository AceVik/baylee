//! Famished Ghoul — {3}{B} — Creature — Zombie
//! Oracle: {1}{B}, Sacrifice this creature: Exile up to two target cards from a single graveyard.
//! Set: ODY #135 — Odyssey | Scryfall ID: e413b65a-700d-44eb-a880-5a0118d2ebac | Oracle ID: 22c74e4b-b1fe-4a35-acd8-fa4840af3c06
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FAMISHED_GHOUL,
    oracle_id = "22c74e4b-b1fe-4a35-acd8-fa4840af3c06",
    scryfall_id = "e413b65a-700d-44eb-a880-5a0118d2ebac",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Famished Ghoul",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
