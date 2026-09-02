//! Balamb Garden, SeeD Academy // Balamb Garden, Airborne — (no cost) — Land — Town // Legendary Artifact — Vehicle
//! Set: FIN #272 — Final Fantasy | Scryfall ID: 001e9f20-5b15-41cb-bf82-46172decc235 | Oracle ID: 8b84fec5-617c-4088-8250-2ba1f1f9479a
//! Face: Balamb Garden, SeeD Academy —  — Land — Town
//! Face: Balamb Garden, Airborne —  — Legendary Artifact — Vehicle
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 253,
    oracle_id: "8b84fec5-617c-4088-8250-2ba1f1f9479a",
    scryfall_id: "001e9f20-5b15-41cb-bf82-46172decc235",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Balamb Garden, SeeD Academy",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Balamb Garden, Airborne",
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::artifact::VEHICLE],
        power: Some(5),
        toughness: Some(4),
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
