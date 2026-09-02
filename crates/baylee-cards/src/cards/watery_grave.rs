//! Watery Grave — (no cost) — Land — ISLAND SWAMP
//! Oracle: ({T}: Add {U} or {B}.)
//! Watery Grave enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #284 — Foundations | Scryfall ID: 5525d6a6-e532-4047-9da4-bfae7927fecc | Oracle ID: fc9ec820-4245-4a96-b009-5308a818ca58
// IMPLEMENTED — shockland (pay 2 life or enters tapped) with the
// two-colour mana ability its type line grants (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 189,
    oracle_id: "fc9ec820-4245-4a96-b009-5308a818ca58",
    scryfall_id: "5525d6a6-e532-4047-9da4-bfae7927fecc",
    faces: &[face! {
        name: "Watery Grave",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])])],
}
