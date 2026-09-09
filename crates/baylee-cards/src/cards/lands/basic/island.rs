//! Island — (no cost) — Basic Land — Island
//! Oracle: ({T}: Add {B}.)
//! Set: TRK #319 — Star Trek | Scryfall ID: f3cc07cd-cc79-4745-b0b7-eade60175cc3 | Oracle ID: b2c6aa39-2d2a-459c-a555-fb48ba993373
// IMPLEMENTED — basic land mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card! {
    index: 74,
    oracle_id: "b2c6aa39-2d2a-459c-a555-fb48ba993373",
    scryfall_id: "f3cc07cd-cc79-4745-b0b7-eade60175cc3",
    faces: &[face! {
        name: "Island",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::ISLAND],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
}
