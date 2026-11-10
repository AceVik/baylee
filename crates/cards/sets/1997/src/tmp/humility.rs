//! Humility — {2}{W}{W} — Enchantment
//! Oracle: All creatures lose all abilities and have base power and toughness 1/1.
//! Set: TPR #16 — Tempest Remastered | Scryfall ID: 55ad6a45-a840-45ba-89ad-066e20e983f3 | Oracle ID: ed7bdb3e-5c51-4547-9266-76a791e0b2b0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUMILITY,
    oracle_id = "ed7bdb3e-5c51-4547-9266-76a791e0b2b0",
    scryfall_id = "55ad6a45-a840-45ba-89ad-066e20e983f3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Humility",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::CREATURE, Modifier::LoseAllAbilities),
        static_ability!(Filter::CREATURE, Modifier::SetPT(1, 1)),
    ],
);
