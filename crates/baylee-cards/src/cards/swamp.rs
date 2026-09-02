//! Swamp — (no cost) — Basic Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Set: TRK #321 — Star Trek | Scryfall ID: b7387103-1df1-4fd0-9e91-1544509792c7 | Oracle ID: 56719f6a-1a6c-4c0a-8d21-18f7d7350b68
// IMPLEMENTED — basic land mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card! {
    index: 162,
    oracle_id: "56719f6a-1a6c-4c0a-8d21-18f7d7350b68",
    scryfall_id: "b7387103-1df1-4fd0-9e91-1544509792c7",
    faces: &[face! {
        name: "Swamp",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::SWAMP],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
}
