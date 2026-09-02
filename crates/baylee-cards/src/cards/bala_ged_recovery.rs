//! Bala Ged Recovery // Bala Ged Sanctuary — {2}{G} — Sorcery // Land
//! Set: ZNR #180 — Zendikar Rising | Scryfall ID: c5cb3052-358d-44a7-8cfd-cd31b236494a | Oracle ID: d2075f58-b0e9-4e85-b7e6-0523a27a1d5b
//! Face: Bala Ged Recovery — {2}{G} — Sorcery
//! Face: Bala Ged Sanctuary —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 252,
    oracle_id: "d2075f58-b0e9-4e85-b7e6-0523a27a1d5b",
    scryfall_id: "c5cb3052-358d-44a7-8cfd-cd31b236494a",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Bala Ged Recovery",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Bala Ged Sanctuary",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
