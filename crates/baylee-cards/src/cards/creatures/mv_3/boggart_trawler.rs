//! Boggart Trawler // Boggart Bog — {2}{B} — Creature — Goblin // Land
//! Set: MH3 #243 — Modern Horizons 3 | Scryfall ID: d0d484a6-5610-4f1d-95ec-eda273c255e4 | Oracle ID: 727f3201-1cfc-4ab2-9dfe-be4f7251f42f
//! Face: Boggart Trawler — {2}{B} — Creature — Goblin
//! Face: Boggart Bog —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 294,
    oracle_id: "727f3201-1cfc-4ab2-9dfe-be4f7251f42f",
    scryfall_id: "d0d484a6-5610-4f1d-95ec-eda273c255e4",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Boggart Trawler",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::GOBLIN],
        power: Some(3),
        toughness: Some(1),
    },
    face! {
        name: "Boggart Bog",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
