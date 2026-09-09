//! Grasping Shadows // Shadows' Lair — {3}{B} — Enchantment // Land — Cave
//! Set: LCI #108 — The Lost Caverns of Ixalan | Scryfall ID: 81b8b9c9-725d-476d-a3cf-55e3dc3e433d | Oracle ID: 522a4b02-24c7-45d2-9097-2803cc9fffad
//! Face: Grasping Shadows — {3}{B} — Enchantment
//! Face: Shadows' Lair —  — Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 571,
    oracle_id: "522a4b02-24c7-45d2-9097-2803cc9fffad",
    scryfall_id: "81b8b9c9-725d-476d-a3cf-55e3dc3e433d",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Grasping Shadows",
        mana_cost: baylee_core::mana!("{3}{B}"),
        types: TypeSet::ENCHANTMENT,
    },
    face! {
        name: "Shadows' Lair",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
