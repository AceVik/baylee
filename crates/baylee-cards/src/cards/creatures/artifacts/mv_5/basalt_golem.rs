//! Basalt Golem — {5} — Artifact Creature — Golem
//! Oracle: This creature can't be blocked by artifact creatures.
//! Oracle: Whenever this creature becomes blocked by a creature, that creature's controller sacrifices it at end of combat. If the player does, they create a 0/2 colorless Wall artifact creature token with defender.
//! Set: MIR #294 — Mirage | Scryfall ID: 5ff538f9-10b4-4327-aea6-a86759daf488 | Oracle ID: 9155f78e-0ccc-4e08-ae79-ef5b2db1183f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BASALT_GOLEM,
    oracle_id = "9155f78e-0ccc-4e08-ae79-ef5b2db1183f",
    scryfall_id = "5ff538f9-10b4-4327-aea6-a86759daf488",
    faces = &[face!(
        name = "Basalt Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
