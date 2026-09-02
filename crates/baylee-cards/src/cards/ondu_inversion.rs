//! Ondu Inversion // Ondu Skyruins — {6}{W}{W} — Sorcery // Land
//! Set: ZNR #30 — Zendikar Rising | Scryfall ID: b6e6be8c-41c3-4348-a8dd-b40ceb24e9b4 | Oracle ID: 15fc4e74-300e-4c2d-8ed7-004553b2f7c2
//! Face: Ondu Inversion — {6}{W}{W} — Sorcery
//! Face: Ondu Skyruins —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 834,
    oracle_id: "15fc4e74-300e-4c2d-8ed7-004553b2f7c2",
    scryfall_id: "b6e6be8c-41c3-4348-a8dd-b40ceb24e9b4",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Ondu Inversion",
        mana_cost: baylee_core::mana!("{6}{W}{W}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Ondu Skyruins",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
