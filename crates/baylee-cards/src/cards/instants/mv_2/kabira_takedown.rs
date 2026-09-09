//! Kabira Takedown // Kabira Plateau — {1}{W} — Instant // Land
//! Set: ZNR #19 — Zendikar Rising | Scryfall ID: 366e9845-019d-47cc-adb8-8fbbaad35b6d | Oracle ID: 0bb73c07-0220-4ba9-8d85-3c357c223833
//! Face: Kabira Takedown — {1}{W} — Instant
//! Face: Kabira Plateau —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 677,
    oracle_id: "0bb73c07-0220-4ba9-8d85-3c357c223833",
    scryfall_id: "366e9845-019d-47cc-adb8-8fbbaad35b6d",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Kabira Takedown",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Kabira Plateau",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
