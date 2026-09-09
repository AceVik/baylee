//! Vastwood Fortification // Vastwood Thicket — {G} — Instant // Land
//! Set: ZNR #216 — Zendikar Rising | Scryfall ID: 3a7fd24e-84d8-405d-86e4-0571a9e23cc2 | Oracle ID: ce148a0c-6c63-49d5-a156-99efae4e367a
//! Face: Vastwood Fortification — {G} — Instant
//! Face: Vastwood Thicket —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1278,
    oracle_id: "ce148a0c-6c63-49d5-a156-99efae4e367a",
    scryfall_id: "3a7fd24e-84d8-405d-86e4-0571a9e23cc2",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Vastwood Fortification",
        mana_cost: baylee_core::mana!("{G}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Vastwood Thicket",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
