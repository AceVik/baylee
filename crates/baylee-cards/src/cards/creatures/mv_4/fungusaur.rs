//! Fungusaur — {3}{G} — Creature — Fungus Dinosaur
//! Oracle: Whenever this creature is dealt damage, put a +1/+1 counter on it.
//! Set: 8ED #250 — Eighth Edition | Scryfall ID: 79324f73-25cd-477a-b4f0-fd3e1319e451 | Oracle ID: 3e771f5b-2de3-4a1b-8281-ab1f7491e5a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FUNGUSAUR,
    oracle_id = "3e771f5b-2de3-4a1b-8281-ab1f7491e5a1",
    scryfall_id = "79324f73-25cd-477a-b4f0-fd3e1319e451",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fungusaur",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FUNGUS, subtypes::creature::DINOSAUR],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
