//! Turntimber Symbiosis // Turntimber, Serpentine Wood — {4}{G}{G}{G} — Sorcery // Land
//! Set: ZNR #215 — Zendikar Rising | Scryfall ID: 61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6 | Oracle ID: 403b59f3-7ade-4bc2-a3e6-de0c3c700f18
//! Face: Turntimber Symbiosis — {4}{G}{G}{G} — Sorcery
//! Face: Turntimber, Serpentine Wood —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1238,
    oracle_id: "403b59f3-7ade-4bc2-a3e6-de0c3c700f18",
    scryfall_id: "61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Turntimber Symbiosis",
        mana_cost: baylee_core::mana!("{4}{G}{G}{G}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Turntimber, Serpentine Wood",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
