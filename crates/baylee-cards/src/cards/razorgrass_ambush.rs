//! Razorgrass Ambush // Razorgrass Field — {1}{W} — Instant // Land
//! Set: MH3 #238 — Modern Horizons 3 | Scryfall ID: 57065dca-f90e-4184-bbc4-95d726a4160b | Oracle ID: 5da954fa-9001-4557-825c-1462035d21ed
//! Face: Razorgrass Ambush — {1}{W} — Instant
//! Face: Razorgrass Field —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 900,
    oracle_id: "5da954fa-9001-4557-825c-1462035d21ed",
    scryfall_id: "57065dca-f90e-4184-bbc4-95d726a4160b",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Razorgrass Ambush",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Razorgrass Field",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
