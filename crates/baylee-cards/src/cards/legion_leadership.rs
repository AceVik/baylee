//! Legion Leadership // Legion Stronghold — {1}{R/W} — Instant // Land
//! Set: MH3 #255 — Modern Horizons 3 | Scryfall ID: 7676abd9-0a3d-4721-b17b-778d2e3c2e25 | Oracle ID: ad225ec2-ff3a-48f6-81a7-dfdd1b75e1f7
//! Face: Legion Leadership — {1}{R/W} — Instant
//! Face: Legion Stronghold —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 709,
    oracle_id: "ad225ec2-ff3a-48f6-81a7-dfdd1b75e1f7",
    scryfall_id: "7676abd9-0a3d-4721-b17b-778d2e3c2e25",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Legion Leadership",
        mana_cost: baylee_core::mana!("{1}{R/W}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Legion Stronghold",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
