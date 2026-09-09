//! Blackbloom Rogue // Blackbloom Bog — {2}{B} — Creature — Human Rogue // Land
//! Set: ZNR #91 — Zendikar Rising | Scryfall ID: 32779721-b021-4bd4-95d1-4a19b78d9faa | Oracle ID: 34320ebf-da97-44a4-bbeb-a9da06548289
//! Face: Blackbloom Rogue — {2}{B} — Creature — Human Rogue
//! Face: Blackbloom Bog —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 274,
    oracle_id: "34320ebf-da97-44a4-bbeb-a9da06548289",
    scryfall_id: "32779721-b021-4bd4-95d1-4a19b78d9faa",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Blackbloom Rogue",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power: Some(2),
        toughness: Some(3),
    },
    face! {
        name: "Blackbloom Bog",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
