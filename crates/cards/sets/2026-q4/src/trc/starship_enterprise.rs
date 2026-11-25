//! Starship Enterprise — {5} — Legendary Artifact — Spacecraft
//! Oracle: Landfall — Whenever a land you control enters, put four charge counters on Starship Enterprise.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 16+.)
//! Oracle: 16+ | Flying
//! Oracle: Whenever Starship Enterprise deals combat damage to a player, draw that many cards.
//! Set: TRC #122 — Star Trek Commander | Scryfall ID: 66fa6995-4857-466d-8068-508f2904805b | Oracle ID: 9b627fa4-af4c-482b-b1cf-2ad15938602c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STARSHIP_ENTERPRISE,
    oracle_id = "9b627fa4-af4c-482b-b1cf-2ad15938602c",
    scryfall_id = "66fa6995-4857-466d-8068-508f2904805b",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Starship Enterprise",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
