//! Island Sanctuary — {1}{W} — Enchantment
//! Oracle: If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.
//! Set: ME4 #15 — Masters Edition IV | Scryfall ID: 5160e700-3bca-4992-a772-48f1f124cee6 | Oracle ID: 7d1769d0-d942-45b3-a31c-2bbe45e68661
// IMPLEMENTED — every draw of the controller's draw step is offered the skip,
// card by card; a skip makes the attack restriction until their next turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ISLAND_SANCTUARY,
    oracle_id = "7d1769d0-d942-45b3-a31c-2bbe45e68661",
    scryfall_id = "5160e700-3bca-4992-a772-48f1f124cee6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Island Sanctuary",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[AbilityDef::Replacement(
        ReplacementRule::MaySkipDrawStepDraw
    )],
);
