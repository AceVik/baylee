//! Arcane Sanctum — (no cost) — Land
//! Oracle: Arcane Sanctum enters the battlefield tapped.
//! {T}: Add White, Blue, or Black.
//! Set: C16 #281 — Commander 2016 | Scryfall ID: c75eeb97-3249-4762-84b0-387f27fb255f | Oracle ID: 7d7cf15c-06b9-4062-a1eb-32614c458a3b
// IMPLEMENTED — 3-color tapland (ETB tapped).

use baylee_cards_dsl::prelude::*;

card! {
    index: 5,
    oracle_id: "7d7cf15c-06b9-4062-a1eb-32614c458a3b",
    scryfall_id: "c75eeb97-3249-4762-84b0-387f27fb255f",
    faces: &[face! {
        name: "Arcane Sanctum",
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
