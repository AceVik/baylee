//! Synthesizer Labship — {U} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 9+.)
//! Oracle: 2+ | At the beginning of combat on your turn, up to one other target artifact you control becomes an artifact creature with base power and toughness 2/2 and gains flying until end of turn.
//! Oracle: 9+ | Flying, vigilance
//! Set: EOE #81 — Edge of Eternities | Scryfall ID: fba6332c-acba-43f9-877c-ca6c5328aae9 | Oracle ID: a37314cf-bbbd-4003-b94d-0e6e60edfc15
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SYNTHESIZER_LABSHIP,
    oracle_id = "a37314cf-bbbd-4003-b94d-0e6e60edfc15",
    scryfall_id = "fba6332c-acba-43f9-877c-ca6c5328aae9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Synthesizer Labship",
        mana_cost = mana!("{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
