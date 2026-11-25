//! Debris Field Crusher — {4}{R} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, it deals 3 damage to any target.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying
//! Oracle: {1}{R}: This Spacecraft gets +2/+0 until end of turn.
//! Set: EOE #131 — Edge of Eternities | Scryfall ID: ea713f35-6442-4388-8839-2714374fb4b6 | Oracle ID: db0894e1-2644-48d4-8de4-8cc43e940bc1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEBRIS_FIELD_CRUSHER,
    oracle_id = "db0894e1-2644-48d4-8de4-8cc43e940bc1",
    scryfall_id = "ea713f35-6442-4388-8839-2714374fb4b6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Debris Field Crusher",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(1),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
