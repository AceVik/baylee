//! Makindi Stampede // Makindi Mesas — {3}{W}{W} — Sorcery // Land
//! Set: ZNR #26 — Zendikar Rising | Scryfall ID: ada9a974-8f1f-4148-bd61-200fc14714b2 | Oracle ID: 342e08f9-d4d0-4408-8621-66e087058616
//! Face: Makindi Stampede — {3}{W}{W} — Sorcery
//! Face: Makindi Mesas —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 736,
    oracle_id: "342e08f9-d4d0-4408-8621-66e087058616",
    scryfall_id: "ada9a974-8f1f-4148-bd61-200fc14714b2",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Makindi Stampede",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Makindi Mesas",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
