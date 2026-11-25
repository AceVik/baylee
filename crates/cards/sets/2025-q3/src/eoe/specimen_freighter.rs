//! Specimen Freighter — {5}{U} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, return up to two target non-Spacecraft creatures to their owners' hands.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 9+.)
//! Oracle: 9+ | Flying
//! Oracle: Whenever this Spacecraft attacks, defending player mills four cards.
//! Set: EOE #76 — Edge of Eternities | Scryfall ID: b862a9f8-2361-4220-99bd-ae2530905195 | Oracle ID: fb84b94c-dc8c-4182-9b96-abd092b20981
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPECIMEN_FREIGHTER,
    oracle_id = "fb84b94c-dc8c-4182-9b96-abd092b20981",
    scryfall_id = "b862a9f8-2361-4220-99bd-ae2530905195",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Specimen Freighter",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
