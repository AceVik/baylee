//! Zof Consumption // Zof Bloodbog — {4}{B}{B} — Sorcery // Land
//! Set: ZNR #132 — Zendikar Rising | Scryfall ID: 98496d5b-1519-4f0c-8b46-0a43be643dfb | Oracle ID: d9f11985-e460-425d-b083-9cb0edf1983a
//! Face: Zof Consumption — {4}{B}{B} — Sorcery
//! Face: Zof Bloodbog —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1343,
    oracle_id: "d9f11985-e460-425d-b083-9cb0edf1983a",
    scryfall_id: "98496d5b-1519-4f0c-8b46-0a43be643dfb",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Zof Consumption",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Zof Bloodbog",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
