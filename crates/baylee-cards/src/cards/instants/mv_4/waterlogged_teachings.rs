//! Waterlogged Teachings // Inundated Archive — {3}{U/B} — Instant // Land
//! Set: MH3 #261 — Modern Horizons 3 | Scryfall ID: 060f9675-4921-4cbb-bae2-54c85c679fd4 | Oracle ID: e6ad1be9-f13d-4590-b3db-e2d0fff46f03
//! Face: Waterlogged Teachings — {3}{U/B} — Instant
//! Face: Inundated Archive —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1310,
    oracle_id: "e6ad1be9-f13d-4590-b3db-e2d0fff46f03",
    scryfall_id: "060f9675-4921-4cbb-bae2-54c85c679fd4",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces: &[
    face! {
        name: "Waterlogged Teachings",
        mana_cost: baylee_core::mana!("{3}{U/B}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Inundated Archive",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
