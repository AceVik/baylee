//! Kazuul's Fury // Kazuul's Cliffs — {2}{R} — Instant // Land
//! Set: ZNR #146 — Zendikar Rising | Scryfall ID: 75240bbc-adc7-48ff-9523-c79776d710d3 | Oracle ID: f8410804-632b-4f18-9a73-6dccc7e4582d
//! Face: Kazuul's Fury — {2}{R} — Instant
//! Face: Kazuul's Cliffs —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 684,
    oracle_id: "f8410804-632b-4f18-9a73-6dccc7e4582d",
    scryfall_id: "75240bbc-adc7-48ff-9523-c79776d710d3",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Kazuul's Fury",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Kazuul's Cliffs",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
