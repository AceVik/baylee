//! Suppression Ray // Orderly Plaza — {3}{W/U}{W/U} — Sorcery // Land
//! Set: MH3 #260 — Modern Horizons 3 | Scryfall ID: 0cccd328-457a-48ab-97fb-4bc319db2e60 | Oracle ID: b592568b-11b0-4081-90a7-30cfb9c1ba80
//! Face: Suppression Ray — {3}{W/U}{W/U} — Sorcery
//! Face: Orderly Plaza —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1121,
    oracle_id: "b592568b-11b0-4081-90a7-30cfb9c1ba80",
    scryfall_id: "0cccd328-457a-48ab-97fb-4bc319db2e60",
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Suppression Ray",
        mana_cost: baylee_core::mana!("{3}{W/U}{W/U}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Orderly Plaza",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
