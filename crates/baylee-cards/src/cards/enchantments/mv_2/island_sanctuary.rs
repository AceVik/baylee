//! Island Sanctuary — {1}{W} — Enchantment
//! Oracle: If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.
//! Set: ME4 #15 — Masters Edition IV | Scryfall ID: 5160e700-3bca-4992-a772-48f1f124cee6 | Oracle ID: 7d1769d0-d942-45b3-a31c-2bbe45e68661
// PARTIAL — skipping a draw in exchange for an attack restriction until your
// next turn is not in the engine.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ISLAND_SANCTUARY,
    oracle_id = "7d1769d0-d942-45b3-a31c-2bbe45e68661",
    scryfall_id = "5160e700-3bca-4992-a772-48f1f124cee6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "skipping a draw in exchange for an attack restriction until your next turn is not in the engine"
    ),
    faces = &[face!(
        name = "Island Sanctuary",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: If you would draw a card during your draw step, instead you may skip
    // that draw. If you do, until your next turn, you can't be attacked except by
    // creatures with flying and/or islandwalk.
);
