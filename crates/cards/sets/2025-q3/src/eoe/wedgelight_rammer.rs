//! Wedgelight Rammer — {3}{W} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, create a 2/2 colorless Robot artifact creature token.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 9+.)
//! Oracle: 9+ | Flying, first strike
//! Set: EOE #43 — Edge of Eternities | Scryfall ID: 2cb0984f-dc8b-4bb3-a4fd-8d6d4ae20198 | Oracle ID: 03259ab0-caa8-4620-b070-127e2c712252
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WEDGELIGHT_RAMMER,
    oracle_id = "03259ab0-caa8-4620-b070-127e2c712252",
    scryfall_id = "2cb0984f-dc8b-4bb3-a4fd-8d6d4ae20198",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wedgelight Rammer",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
