//! Warmaker Gunship — {2}{R} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, it deals damage equal to the number of artifacts you control to target creature an opponent controls.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying
//! Set: EOE #167 — Edge of Eternities | Scryfall ID: 9e5957f4-1cae-4989-8b40-27fc6e2fcf5e | Oracle ID: c947171b-ed9e-4b83-af45-bd595a8d84ee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARMAKER_GUNSHIP,
    oracle_id = "c947171b-ed9e-4b83-af45-bd595a8d84ee",
    scryfall_id = "9e5957f4-1cae-4989-8b40-27fc6e2fcf5e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Warmaker Gunship",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
