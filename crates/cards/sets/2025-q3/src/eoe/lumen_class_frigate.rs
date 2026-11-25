//! Lumen-Class Frigate — {1}{W} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 12+.)
//! Oracle: 2+ | Other creatures you control get +1/+1.
//! Oracle: 12+ | Flying, lifelink
//! Set: EOE #25 — Edge of Eternities | Scryfall ID: 0cc59b5a-65fa-47cc-8ac7-b7c3f533a782 | Oracle ID: 3aa0cda4-fb2d-4fd4-84f6-482089a85547
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LUMEN_CLASS_FRIGATE,
    oracle_id = "3aa0cda4-fb2d-4fd4-84f6-482089a85547",
    scryfall_id = "0cc59b5a-65fa-47cc-8ac7-b7c3f533a782",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Lumen-Class Frigate",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
