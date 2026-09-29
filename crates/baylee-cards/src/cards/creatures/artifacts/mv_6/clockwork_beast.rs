//! Clockwork Beast — {6} — Artifact Creature — Beast
//! Oracle: This creature enters with seven +1/+0 counters on it.
//! Oracle: At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it.
//! Oracle: {X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven. Activate only during your upkeep.
//! Set: ME1 #153 — Masters Edition | Scryfall ID: 1a3bdfda-4269-45be-931d-ecfecbb389a8 | Oracle ID: eb97c8db-ac6c-476c-b14d-87785e9c82f0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLOCKWORK_BEAST,
    oracle_id = "eb97c8db-ac6c-476c-b14d-87785e9c82f0",
    scryfall_id = "1a3bdfda-4269-45be-931d-ecfecbb389a8",
    faces = &[face!(
        name = "Clockwork Beast",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
