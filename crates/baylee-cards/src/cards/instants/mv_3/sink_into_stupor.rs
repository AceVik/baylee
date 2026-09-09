//! Sink into Stupor // Soporific Springs — {1}{U}{U} — Instant // Land
//! Set: MH3 #241 — Modern Horizons 3 | Scryfall ID: 5358b87a-1a29-426d-b165-40c97da2c14d | Oracle ID: bcc6eece-75ea-494c-b33a-d4477d504e0b
//! Face: Sink into Stupor — {1}{U}{U} — Instant
//! Face: Soporific Springs —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1038,
    oracle_id: "bcc6eece-75ea-494c-b33a-d4477d504e0b",
    scryfall_id: "5358b87a-1a29-426d-b165-40c97da2c14d",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Sink into Stupor",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Soporific Springs",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
