//! Agadeem's Awakening // Agadeem, the Undercrypt — {X}{B}{B}{B} — Sorcery // Land
//! Set: ZNR #90 — Zendikar Rising | Scryfall ID: 67f4c93b-080c-4196-b095-6a120a221988 | Oracle ID: 562d71b9-1646-474e-9293-55da6947a758
//! Face: Agadeem's Awakening — {X}{B}{B}{B} — Sorcery
//! Face: Agadeem, the Undercrypt —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 211,
    oracle_id: "562d71b9-1646-474e-9293-55da6947a758",
    scryfall_id: "67f4c93b-080c-4196-b095-6a120a221988",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Agadeem's Awakening",
        mana_cost: baylee_core::mana!("{X}{B}{B}{B}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Agadeem, the Undercrypt",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
