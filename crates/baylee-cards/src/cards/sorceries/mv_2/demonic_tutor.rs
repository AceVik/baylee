//! Demonic Tutor — {1}{B} — Sorcery
//! Oracle: Search your library for a card, put that card into your hand, then shuffle.
//! Set: CMM #150 — Commander Masters | Scryfall ID: a24b4cb6-cebb-428b-8654-74347a6a8d63 | Oracle ID: 82004860-e589-4e38-8d61-8c0210e4ea39
// IMPLEMENTED — unrestricted tutor to hand.

use baylee_cards_dsl::prelude::*;

card! {
    index: 32,
    oracle_id: "82004860-e589-4e38-8d61-8c0210e4ea39",
    scryfall_id: "a24b4cb6-cebb-428b-8654-74347a6a8d63",
    faces: &[face! {
        name: "Demonic Tutor",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::SearchLibrary {
            filter: &Filter::Any,
            finds: &[Find::HAND],
            optional: false,
        }])],
}

// Engine-level coverage via s4 scenario tests: tutoring puts any chosen
// library card into hand and shuffles.
