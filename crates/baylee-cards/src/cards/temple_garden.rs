//! Temple Garden — (no cost) — Land — FOREST PLAINS
//! Oracle: ({T}: Add {G} or {W}.)
//! Temple Garden enters the battlefield tapped unless you pay 2 life.
//! Set: FDN #283 — Foundations | Scryfall ID: b9b0589d-f327-46a7-8bac-06b7654c547a | Oracle ID: f413a83d-a40d-434c-b20a-4c707c0527fa
// IMPLEMENTED — shockland (pay 2 life or enters tapped) with the
// two-colour mana ability its type line grants (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 167,
    oracle_id: "f413a83d-a40d-434c-b20a-4c707c0527fa",
    scryfall_id: "b9b0589d-f327-46a7-8bac-06b7654c547a",
    faces: &[face! {
        name: "Temple Garden",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        enter_modifiers: &[EnterModifier::TappedOrPayLife(2)],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])])],
}
