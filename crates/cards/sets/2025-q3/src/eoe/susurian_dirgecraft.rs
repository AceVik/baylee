//! Susurian Dirgecraft — {4}{B} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, each opponent sacrifices a nontoken creature of their choice.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Set: EOE #117 — Edge of Eternities | Scryfall ID: b67cdb6e-9a3b-4887-924d-318faa3c443d | Oracle ID: f1d826d1-d2da-46ce-b1db-81393c88d865
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUSURIAN_DIRGECRAFT,
    oracle_id = "f1d826d1-d2da-46ce-b1db-81393c88d865",
    scryfall_id = "b67cdb6e-9a3b-4887-924d-318faa3c443d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Susurian Dirgecraft",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
