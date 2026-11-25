//! Galvanizing Sawship — {5}{R} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 3+.)
//! Oracle: 3+ | Flying, haste
//! Set: EOE #136 — Edge of Eternities | Scryfall ID: 5bbce9fb-401f-4e78-acd5-9d3b506687fd | Oracle ID: dfe8f77a-cc26-438b-92ae-2ca7a91f813b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GALVANIZING_SAWSHIP,
    oracle_id = "dfe8f77a-cc26-438b-92ae-2ca7a91f813b",
    scryfall_id = "5bbce9fb-401f-4e78-acd5-9d3b506687fd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Galvanizing Sawship",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
