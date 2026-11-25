//! Atmospheric Greenhouse — {4}{G} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, put a +1/+1 counter on each creature you control.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying, trample
//! Set: EOE #171 — Edge of Eternities | Scryfall ID: bf05e378-7a0c-49e3-8c6e-c0fd56796434 | Oracle ID: 52c6afc6-aff1-46a8-b1ea-b183facec1ae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ATMOSPHERIC_GREENHOUSE,
    oracle_id = "52c6afc6-aff1-46a8-b1ea-b183facec1ae",
    scryfall_id = "bf05e378-7a0c-49e3-8c6e-c0fd56796434",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Atmospheric Greenhouse",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
