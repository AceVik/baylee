//! Lindblum, Industrial Regency // Mage Siege — (no cost) — Land — Town // Instant — Adventure
//! Set: FIN #285 — Final Fantasy | Scryfall ID: 548dd152-f0b6-4e8f-9afc-a4ec1671b648 | Oracle ID: 4cc014f3-05e0-442e-9dee-03eab1aa65a3
//! Face: Lindblum, Industrial Regency —  — Land — Town
//! Face: Mage Siege — {2}{R} — Instant — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 714,
    oracle_id: "4cc014f3-05e0-442e-9dee-03eab1aa65a3",
    scryfall_id: "548dd152-f0b6-4e8f-9afc-a4ec1671b648",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Lindblum, Industrial Regency",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Mage Siege",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::INSTANT,
        subtypes: &[subtypes::spell::ADVENTURE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
