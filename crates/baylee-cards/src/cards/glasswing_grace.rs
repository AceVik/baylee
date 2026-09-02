//! Glasswing Grace // Age-Graced Chapel — {3}{W/B}{W/B} — Enchantment — Aura // Land
//! Set: MH3 #254 — Modern Horizons 3 | Scryfall ID: 90630b20-fc83-475f-bcd5-8bcfee0cf241 | Oracle ID: 3a3e8c9b-e458-4661-980d-0a84a4c2452b
//! Face: Glasswing Grace — {3}{W/B}{W/B} — Enchantment — Aura
//! Face: Age-Graced Chapel —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 552,
    oracle_id: "3a3e8c9b-e458-4661-980d-0a84a4c2452b",
    scryfall_id: "90630b20-fc83-475f-bcd5-8bcfee0cf241",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Glasswing Grace",
        mana_cost: baylee_core::mana!("{3}{W/B}{W/B}"),
        types: TypeSet::ENCHANTMENT,
        subtypes: &[subtypes::enchantment::AURA],
    },
    face! {
        name: "Age-Graced Chapel",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
