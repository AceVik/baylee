//! Canopy Vista — (no cost) — Land — Forest Plains
//! Oracle: ({T}: Add {G} or {W}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #227 — Marvel Super Heroes Commander | Scryfall ID: 369e6485-fbc7-45f9-ac9d-2e26dd94f2d5 | Oracle ID: dcb7e046-f01b-497c-88e5-57794eb30ce5
// PARTIAL — mana ability implemented; ETB count condition (≥2 basic lands)
// inexpressible — TappedUnless only checks existence (≥1), not a count.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 326,
    oracle_id: "dcb7e046-f01b-497c-88e5-57794eb30ce5",
    scryfall_id: "369e6485-fbc7-45f9-ac9d-2e26dd94f2d5",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[face! {
        name: "Canopy Vista",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        // NOT SUPPORTED: "enters tapped unless you control two or more basic
        // lands" requires a count-based ETB condition (≥2 matching permanents).
        // EnterModifier::TappedUnless checks only for existence (≥1 matching
        // permanent) via .any(); no count variant exists in the DSL yet.
    }],
    coverage: Coverage::Partial("ETB tapped-unless count (≥2 basic lands) inexpressible: TappedUnless checks existence only"),
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])])],
}
