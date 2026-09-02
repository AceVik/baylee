//! Beyeen Veil // Beyeen Coast — {1}{U} — Instant // Land
//! Set: ZNR #46 — Zendikar Rising | Scryfall ID: 5f411f08-45dd-4d73-8894-daf51c175150 | Oracle ID: b03de49d-246f-44e2-9487-9e4e43ec7be4
//! Face: Beyeen Veil — {1}{U} — Instant
//! Face: Beyeen Coast —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 269,
    oracle_id: "b03de49d-246f-44e2-9487-9e4e43ec7be4",
    scryfall_id: "5f411f08-45dd-4d73-8894-daf51c175150",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Beyeen Veil",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Beyeen Coast",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
