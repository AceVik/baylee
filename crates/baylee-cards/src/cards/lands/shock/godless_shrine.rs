//! Godless Shrine — (no cost) — Land — PLAINS SWAMP
//! Oracle: ({T}: Add {W} or {B}.)
//! Godless Shrine enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #281 — Foundations | Scryfall ID: 8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6 | Oracle ID: 73864fcc-1bde-4bc0-831e-2b93e546e417
// IMPLEMENTED — shockland (pay 2 life or enters tapped) with the
// two-colour mana ability its type line grants (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 61,
    oracle_id: "73864fcc-1bde-4bc0-831e-2b93e546e417",
    scryfall_id: "8fbd1ae0-3d4c-492a-a1ea-85a95fa3d7b6",
    faces: &[face! {
        name: "Godless Shrine",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Black])])],
}
