//! Kazandu Mammoth // Kazandu Valley — {1}{G}{G} — Creature — Elephant // Land
//! Set: ZNR #189 — Zendikar Rising | Scryfall ID: 2f632537-63bf-4490-86e6-e6067b9c1a3b | Oracle ID: 2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7
//! Face: Kazandu Mammoth — {1}{G}{G} — Creature — Elephant
//! Face: Kazandu Valley —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 682,
    oracle_id: "2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7",
    scryfall_id: "2f632537-63bf-4490-86e6-e6067b9c1a3b",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Kazandu Mammoth",
        mana_cost: baylee_core::mana!("{1}{G}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::ELEPHANT],
        power: Some(3),
        toughness: Some(3),
    },
    face! {
        name: "Kazandu Valley",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
