//! Mystical Tutor — {U} — Instant
//! Oracle: Search your library for an instant or sorcery card, reveal it, then shuffle and put that card on top.
//! Set: DMR #60 — Dominaria Remastered | Scryfall ID: 36fa9a0b-b0c9-43ea-ba11-99d7982f974e | Oracle ID: fb81f95c-70f8-4eb7-8d15-15d0ae23ec03
// IMPLEMENTED — filtered tutor to the top of the library (reveal is M3).

use baylee_cards_dsl::prelude::*;

card! {
    index: 102,
    oracle_id: "fb81f95c-70f8-4eb7-8d15-15d0ae23ec03",
    scryfall_id: "36fa9a0b-b0c9-43ea-ba11-99d7982f974e",
    faces: &[face! {
        name: "Mystical Tutor",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::SearchLibrary {
            filter: &Filter::INSTANT_OR_SORCERY,
            finds: &[Find::TOP_OF_LIBRARY],
            optional: false,
        }])],
}
