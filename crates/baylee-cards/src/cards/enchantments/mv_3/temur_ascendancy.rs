//! Temur Ascendancy — {G}{U}{R} — Enchantment
//! Oracle: Creatures you control have haste.
//! Oracle: Whenever a creature you control with power 4 or greater enters, you may draw a card.
//! Set: TDC #305 — Tarkir: Dragonstorm Commander | Scryfall ID: 5cedb54a-a6f6-48aa-acdf-01c988c1c37d | Oracle ID: e68dc47c-692f-4420-9799-eee104017273
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMUR_ASCENDANCY,
    oracle_id = "e68dc47c-692f-4420-9799-eee104017273",
    scryfall_id = "5cedb54a-a6f6-48aa-acdf-01c988c1c37d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Temur Ascendancy",
        mana_cost = mana!("{G}{U}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
