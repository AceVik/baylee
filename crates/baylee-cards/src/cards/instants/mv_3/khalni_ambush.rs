//! Khalni Ambush // Khalni Territory — {2}{G} — Instant // Land
//! Set: ZNR #192 — Zendikar Rising | Scryfall ID: 99535539-aa73-41ed-86ab-21c97b92620d | Oracle ID: 37a55560-6e32-4f54-b9a8-fd157aea6eb5
//! Face: Khalni Ambush — {2}{G} — Instant
//! Face: Khalni Territory —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 689,
    oracle_id: "37a55560-6e32-4f54-b9a8-fd157aea6eb5",
    scryfall_id: "99535539-aa73-41ed-86ab-21c97b92620d",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Khalni Ambush",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Khalni Territory",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
