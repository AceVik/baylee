//! Silundi Vision // Silundi Isle — {2}{U} — Instant // Land
//! Set: ZNR #80 — Zendikar Rising | Scryfall ID: 11568cdf-6148-494c-8b98-f5ca5797d775 | Oracle ID: b0182ca0-f353-4012-9121-6f4ac9f7a046
//! Face: Silundi Vision — {2}{U} — Instant
//! Face: Silundi Isle —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1032,
    oracle_id: "b0182ca0-f353-4012-9121-6f4ac9f7a046",
    scryfall_id: "11568cdf-6148-494c-8b98-f5ca5797d775",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Silundi Vision",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Silundi Isle",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
