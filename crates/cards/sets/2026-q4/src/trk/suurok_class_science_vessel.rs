//! Suurok-Class Science Vessel — {2}{U} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, draw a card.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 5+.)
//! Oracle: 5+ | Flying
//! Set: TRK #83 — Star Trek | Scryfall ID: 7608943e-329e-41f9-b098-a0a94b01e6a1 | Oracle ID: 5d2016ed-3ea0-4938-b95e-ee582307e475
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUUROK_CLASS_SCIENCE_VESSEL,
    oracle_id = "5d2016ed-3ea0-4938-b95e-ee582307e475",
    scryfall_id = "7608943e-329e-41f9-b098-a0a94b01e6a1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Suurok-Class Science Vessel",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
