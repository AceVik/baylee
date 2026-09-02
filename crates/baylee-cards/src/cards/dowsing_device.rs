//! Dowsing Device // Geode Grotto — {1}{R} — Artifact // Land — Cave
//! Set: LCI #146 — The Lost Caverns of Ixalan | Scryfall ID: 3d715e9f-223d-462e-8ce3-eebbaf1cd021 | Oracle ID: 2f4374f6-c695-4a5d-a6d6-0e41eaa587ca
//! Face: Dowsing Device — {1}{R} — Artifact
//! Face: Geode Grotto —  — Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 434,
    oracle_id: "2f4374f6-c695-4a5d-a6d6-0e41eaa587ca",
    scryfall_id: "3d715e9f-223d-462e-8ce3-eebbaf1cd021",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Dowsing Device",
        mana_cost: baylee_core::mana!("{1}{R}"),
        types: TypeSet::ARTIFACT,
    },
    face! {
        name: "Geode Grotto",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
