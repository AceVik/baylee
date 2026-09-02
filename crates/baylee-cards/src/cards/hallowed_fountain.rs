//! Hallowed Fountain — (no cost) — Land — PLAINS ISLAND
//! Oracle: ({T}: Add {W} or {U}.)
//! Hallowed Fountain enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #280 — Foundations | Scryfall ID: b7285986-7e08-4969-86ef-452dc5bfdd9f | Oracle ID: f1750962-a87c-49f6-b731-02ae971ac6ea
// IMPLEMENTED — shockland (pay 2 life or enters tapped) with the
// two-colour mana ability its type line grants (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 65,
    oracle_id: "f1750962-a87c-49f6-b731-02ae971ac6ea",
    scryfall_id: "b7285986-7e08-4969-86ef-452dc5bfdd9f",
    faces: &[face! {
        name: "Hallowed Fountain",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])],
}
