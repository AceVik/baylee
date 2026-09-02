//! Forest — (no cost) — Basic Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Set: TRK #325 — Star Trek | Scryfall ID: dce15387-4114-4b3e-91aa-5b42b45c44ac | Oracle ID: b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6
// IMPLEMENTED — basic land mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card! {
    index: 55,
    oracle_id: "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6",
    scryfall_id: "dce15387-4114-4b3e-91aa-5b42b45c44ac",
    faces: &[face! {
        name: "Forest",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::FOREST],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
}
