//! Dodecapod — {4} — Artifact Creature — Golem
//! Oracle: If a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield with two +1/+1 counters on it instead of putting it into your graveyard.
//! Set: DMR #220 — Dominaria Remastered | Scryfall ID: 156afb5a-df38-4555-b59b-f3fa64ce5bff | Oracle ID: 2c189e3b-90d3-49b4-bebe-a4f14c8a275b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DODECAPOD,
    oracle_id = "2c189e3b-90d3-49b4-bebe-a4f14c8a275b",
    scryfall_id = "156afb5a-df38-4555-b59b-f3fa64ce5bff",
    faces = &[face!(
        name = "Dodecapod",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
