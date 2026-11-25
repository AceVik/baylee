//! Deep Space 9 — {9} — Legendary Artifact — Spacecraft
//! Oracle: When Deep Space 9 enters and at the beginning of your upkeep, exile the top four cards of your library. Put all permanent cards exiled this way onto the battlefield.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery.)
//! Oracle: 9+ | Creatures you control get +3/+3.
//! Set: TRK #263 — Star Trek | Scryfall ID: 76b039c5-e0a1-410a-954b-52f304e31f7f | Oracle ID: 14442f5b-0bad-4c58-b268-d775787571d8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEEP_SPACE_9,
    oracle_id = "14442f5b-0bad-4c58-b268-d775787571d8",
    scryfall_id = "76b039c5-e0a1-410a-954b-52f304e31f7f",
    faces = &[face!(
        name = "Deep Space 9",
        mana_cost = mana!("{9}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
