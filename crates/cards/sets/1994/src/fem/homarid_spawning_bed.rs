//! Homarid Spawning Bed — {U}{U} — Enchantment
//! Oracle: {1}{U}{U}, Sacrifice a blue creature: Create X 1/1 blue Camarid creature tokens, where X is the sacrificed creature's mana value.
//! Set: ME1 #36 — Masters Edition | Scryfall ID: 16e9392f-195d-4d4a-95c8-ef4597b5e26b | Oracle ID: 173f2b8e-8072-4fdd-a469-8a1fb0bb6e64
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

const YOUR_BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
    Filter::ControlledByYou,
]);

card!(
    index = index::HOMARID_SPAWNING_BED,
    oracle_id = "173f2b8e-8072-4fdd-a469-8a1fb0bb6e64",
    scryfall_id = "16e9392f-195d-4d4a-95c8-ef4597b5e26b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Homarid Spawning Bed",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}{U}{U}", Sacrifice(&YOUR_BLUE_CREATURE)),
        &[Effect::CreateTokenN {
            token: &generated_tokens::CAMARID_1_1_BLUE,
            amount: Amount::SacrificedManaValue,
        }],
    ),],
);
