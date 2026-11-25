//! Wurmwall Sweeper — {2} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, surveil 2.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 4+.)
//! Oracle: 4+ | Flying
//! Set: EOE #249 — Edge of Eternities | Scryfall ID: 9ace282a-5901-4d36-ad21-17eb88bc5138 | Oracle ID: 320ed2ec-ef06-41fb-ac8a-72579f11544c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WURMWALL_SWEEPER,
    oracle_id = "320ed2ec-ef06-41fb-ac8a-72579f11544c",
    scryfall_id = "9ace282a-5901-4d36-ad21-17eb88bc5138",
    faces = &[face!(
        name = "Wurmwall Sweeper",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
