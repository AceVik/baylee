//! Song-Mad Treachery // Song-Mad Ruins — {3}{R}{R} — Sorcery // Land
//! Set: ZNR #165 — Zendikar Rising | Scryfall ID: 782ca27f-9f18-476c-b582-89c06fb2e322 | Oracle ID: 81b61770-2ed5-4a50-84d0-97790002fc5a
//! Face: Song-Mad Treachery — {3}{R}{R} — Sorcery
//! Face: Song-Mad Ruins —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1065,
    oracle_id: "81b61770-2ed5-4a50-84d0-97790002fc5a",
    scryfall_id: "782ca27f-9f18-476c-b582-89c06fb2e322",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Song-Mad Treachery",
        mana_cost: baylee_core::mana!("{3}{R}{R}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Song-Mad Ruins",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
