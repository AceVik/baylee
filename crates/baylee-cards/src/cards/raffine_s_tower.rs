//! Raffine's Tower — (no cost) — Land
//! Oracle: Raffine's Tower enters the battlefield tapped.
//! {T}: Add White, Blue, or Black.
//! Set: SNC #254 — Streets of New Capenna | Scryfall ID: a2c56479-4bee-4edb-80d7-4af010b7c793 | Oracle ID: 6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b
// IMPLEMENTED — 3-color tapland (ETB tapped).

use baylee_cards_dsl::prelude::*;

card! {
    index: 122,
    oracle_id: "6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b",
    scryfall_id: "a2c56479-4bee-4edb-80d7-4af010b7c793",
    faces: &[face! {
        name: "Raffine's Tower",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
        ])])],
}
