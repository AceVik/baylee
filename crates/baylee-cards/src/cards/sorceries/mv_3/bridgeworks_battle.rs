//! Bridgeworks Battle // Tanglespan Bridgeworks — {2}{G} — Sorcery // Land
//! Set: MH3 #249 — Modern Horizons 3 | Scryfall ID: ebef3db0-2b58-4581-a79c-fbca9a059e63 | Oracle ID: 9d581188-ce80-494e-bd38-f411e1f4efb5
//! Face: Bridgeworks Battle — {2}{G} — Sorcery
//! Face: Tanglespan Bridgeworks —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 312,
    oracle_id: "9d581188-ce80-494e-bd38-f411e1f4efb5",
    scryfall_id: "ebef3db0-2b58-4581-a79c-fbca9a059e63",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Bridgeworks Battle",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Tanglespan Bridgeworks",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
