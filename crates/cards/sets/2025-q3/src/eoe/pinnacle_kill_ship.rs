//! Pinnacle Kill-Ship — {7} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, it deals 10 damage to up to one target creature.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Set: EOE #244 — Edge of Eternities | Scryfall ID: bf784de8-5ae2-4c07-92bb-a5b7f593b773 | Oracle ID: ad556aee-3dbf-4c8b-9f3a-31947e26c6f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PINNACLE_KILL_SHIP,
    oracle_id = "ad556aee-3dbf-4c8b-9f3a-31947e26c6f5",
    scryfall_id = "bf784de8-5ae2-4c07-92bb-a5b7f593b773",
    faces = &[face!(
        name = "Pinnacle Kill-Ship",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(7),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
