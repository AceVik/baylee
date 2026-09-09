//! Pelakka Predation // Pelakka Caverns — {2}{B} — Sorcery // Land
//! Set: ZNR #120 — Zendikar Rising | Scryfall ID: e63f8b20-f45b-4293-9aac-cdc021939be6 | Oracle ID: b0fd6889-20b4-439b-aa97-2e90aca1675a
//! Face: Pelakka Predation — {2}{B} — Sorcery
//! Face: Pelakka Caverns —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 852,
    oracle_id: "b0fd6889-20b4-439b-aa97-2e90aca1675a",
    scryfall_id: "e63f8b20-f45b-4293-9aac-cdc021939be6",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Pelakka Predation",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Pelakka Caverns",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
