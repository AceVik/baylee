//! Tangled Florahedron // Tangled Vale — {1}{G} — Creature — Elemental // Land
//! Set: ZNR #211 — Zendikar Rising | Scryfall ID: 235d1ffc-72aa-40a2-95dc-3f6a8d495061 | Oracle ID: 53542c79-a62a-4d6a-97db-5296e9c68302
//! Face: Tangled Florahedron — {1}{G} — Creature — Elemental
//! Face: Tangled Vale —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1137,
    oracle_id: "53542c79-a62a-4d6a-97db-5296e9c68302",
    scryfall_id: "235d1ffc-72aa-40a2-95dc-3f6a8d495061",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Tangled Florahedron",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::ELEMENTAL],
        power: Some(1),
        toughness: Some(1),
    },
    face! {
        name: "Tangled Vale",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
