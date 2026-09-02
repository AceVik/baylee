//! Sejiri Shelter // Sejiri Glacier — {1}{W} — Instant // Land
//! Set: ZNR #37 — Zendikar Rising | Scryfall ID: f25d56f9-aa54-4657-9ac9-e93fbba3e715 | Oracle ID: d54e4e37-042b-44a5-918d-757308545d4d
//! Face: Sejiri Shelter — {1}{W} — Instant
//! Face: Sejiri Glacier —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 998,
    oracle_id: "d54e4e37-042b-44a5-918d-757308545d4d",
    scryfall_id: "f25d56f9-aa54-4657-9ac9-e93fbba3e715",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Sejiri Shelter",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Sejiri Glacier",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
