//! Borg Scout Sphere — {2}{U} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, draw a card.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Artifact creatures you control have flying.
//! Set: TRC #36 — Star Trek Commander | Scryfall ID: 0813b13e-06a3-403b-ac00-6d6edd74935d | Oracle ID: 4cfb6f06-137c-4b41-97ef-6710e899473b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BORG_SCOUT_SPHERE,
    oracle_id = "4cfb6f06-137c-4b41-97ef-6710e899473b",
    scryfall_id = "0813b13e-06a3-403b-ac00-6d6edd74935d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Borg Scout Sphere",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
