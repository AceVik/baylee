//! Icehide Golem — {S} — Snow Artifact Creature — Golem
//! Oracle: ({S} can be paid with one mana from a snow source.)
//! Set: MH1 #224 — Modern Horizons | Scryfall ID: b2671c36-96fa-49e3-90dc-52820b2c8a62 | Oracle ID: 29b925e9-1e98-471c-9b23-e0372ac86485
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICEHIDE_GOLEM,
    oracle_id = "29b925e9-1e98-471c-9b23-e0372ac86485",
    scryfall_id = "b2671c36-96fa-49e3-90dc-52820b2c8a62",
    faces = &[face!(
        name = "Icehide Golem",
        mana_cost = mana!("{S}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::SNOW,
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
