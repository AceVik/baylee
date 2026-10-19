//! Goblin Warrens — {2}{R} — Enchantment
//! Oracle: {2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens.
//! Set: ME4 #123 — Masters Edition IV | Scryfall ID: 2d8ce8ad-2bb9-4c53-8e36-9690e8a118f1 | Oracle ID: c1161505-1623-4475-a88e-2e48072cc2f1
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_GOBLIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::GOBLIN_WARRENS,
    oracle_id = "c1161505-1623-4475-a88e-2e48072cc2f1",
    scryfall_id = "2d8ce8ad-2bb9-4c53-8e36-9690e8a118f1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Warrens",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{R}", Sacrifice(&YOUR_GOBLIN), Sacrifice(&YOUR_GOBLIN)),
        &[Effect::CreateTokenN {
            token: &generated_tokens::GOBLIN_1_1_RED,
            amount: Amount::Fixed(3),
        }],
    ),],
);
