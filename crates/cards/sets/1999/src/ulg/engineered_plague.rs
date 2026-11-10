//! Engineered Plague — {2}{B} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: All creatures of the chosen type get -1/-1.
//! Set: 7ED #133 — Seventh Edition | Scryfall ID: b669e43e-3b11-42c9-8f20-0acce129e63c | Oracle ID: dc8210b4-8e17-4156-994b-e95e215e3e34
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENGINEERED_PLAGUE,
    oracle_id = "dc8210b4-8e17-4156-994b-e95e215e3e34",
    scryfall_id = "b669e43e-3b11-42c9-8f20-0acce129e63c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Engineered Plague",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]),
        Modifier::ModifyPT(-1, -1),
    )],
);
