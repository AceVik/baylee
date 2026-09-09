//! Jwari Disruption // Jwari Ruins — {1}{U} — Instant // Land
//! Set: ZNR #64 — Zendikar Rising | Scryfall ID: 301750a7-d1fd-435e-bfa8-9d2fb22ad627 | Oracle ID: 941a4b14-ea2a-4bd0-8cc2-d609f80df32c
//! Face: Jwari Disruption — {1}{U} — Instant
//! Face: Jwari Ruins —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 675,
    oracle_id: "941a4b14-ea2a-4bd0-8cc2-d609f80df32c",
    scryfall_id: "301750a7-d1fd-435e-bfa8-9d2fb22ad627",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Jwari Disruption",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Jwari Ruins",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
