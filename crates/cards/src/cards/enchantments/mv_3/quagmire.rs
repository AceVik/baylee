//! Quagmire — {2}{B} — Enchantment
//! Oracle: Creatures with swampwalk can be blocked as though they didn't have swampwalk.
//! Set: LEG #115 — Legends | Scryfall ID: 94e2aa9e-af6a-41c6-99a8-ca9335730ddb | Oracle ID: 0fb54ed7-4c64-4029-b52d-2bf5343e7426
// IMPLEMENTED — a layer-6 static strips swampwalk from every creature that
// has it, so it can be blocked as though it didn't have swampwalk.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::QUAGMIRE,
    oracle_id = "0fb54ed7-4c64-4029-b52d-2bf5343e7426",
    scryfall_id = "94e2aa9e-af6a-41c6-99a8-ca9335730ddb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Quagmire",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SWAMPWALK)]),
        Modifier::RemoveKeyword(KeywordSet::SWAMPWALK)
    )],
);
