//! Oberth-Class Starship — {1}{W} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying
//! Set: TRK #26 — Star Trek | Scryfall ID: 7efe3788-ecf7-4b56-8970-bc324092941c | Oracle ID: fb5e53cf-e12e-469b-b9ea-9debc967e77e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OBERTH_CLASS_STARSHIP,
    oracle_id = "fb5e53cf-e12e-469b-b9ea-9debc967e77e",
    scryfall_id = "7efe3788-ecf7-4b56-8970-bc324092941c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Oberth-Class Starship",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
