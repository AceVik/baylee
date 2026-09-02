//! Sea Gate Restoration // Sea Gate, Reborn — {4}{U}{U}{U} — Sorcery // Land
//! Set: ZNR #76 — Zendikar Rising | Scryfall ID: 193071fe-180b-4d35-ba78-9c16675c29fc | Oracle ID: 4a8d41fe-e04d-484b-a7d1-19be311e6ca7
//! Face: Sea Gate Restoration — {4}{U}{U}{U} — Sorcery
//! Face: Sea Gate, Reborn —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 979,
    oracle_id: "4a8d41fe-e04d-484b-a7d1-19be311e6ca7",
    scryfall_id: "193071fe-180b-4d35-ba78-9c16675c29fc",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Sea Gate Restoration",
        mana_cost: baylee_core::mana!("{4}{U}{U}{U}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Sea Gate, Reborn",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
