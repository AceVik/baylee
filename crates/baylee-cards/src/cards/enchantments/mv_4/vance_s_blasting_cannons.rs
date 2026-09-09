//! Vance's Blasting Cannons // Spitfire Bastion — {3}{R} — Legendary Enchantment // Legendary Land
//! Set: XLN #173 — Ixalan | Scryfall ID: 9e8c0009-787f-480b-84b6-bf297f1fb466 | Oracle ID: 5e7eca9c-a7b8-4b7b-a0a0-e8937530145a
//! Face: Vance's Blasting Cannons — {3}{R} — Legendary Enchantment
//! Face: Spitfire Bastion —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1277,
    oracle_id: "5e7eca9c-a7b8-4b7b-a0a0-e8937530145a",
    scryfall_id: "9e8c0009-787f-480b-84b6-bf297f1fb466",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Vance's Blasting Cannons",
        mana_cost: baylee_core::mana!("{3}{R}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Spitfire Bastion",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
