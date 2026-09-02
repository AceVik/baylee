//! Fell the Profane // Fell Mire — {2}{B}{B} — Instant // Land
//! Set: MH3 #244 — Modern Horizons 3 | Scryfall ID: a3cb782d-c459-468d-9779-9b5669abc337 | Oracle ID: 053a69d8-2b5e-4f14-8b02-ca405891dc4a
//! Face: Fell the Profane — {2}{B}{B} — Instant
//! Face: Fell Mire —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 485,
    oracle_id: "053a69d8-2b5e-4f14-8b02-ca405891dc4a",
    scryfall_id: "a3cb782d-c459-468d-9779-9b5669abc337",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Fell the Profane",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Fell Mire",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
