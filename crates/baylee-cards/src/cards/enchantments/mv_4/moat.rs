//! Moat — {2}{W}{W} — Enchantment
//! Oracle: Creatures without flying can't attack.
//! Set: ME1 #21 — Masters Edition | Scryfall ID: e2dffeb3-c858-4b8c-ae1f-109721f7d2da | Oracle ID: 42208fea-8c24-451f-861d-6d70c0a7a502
// IMPLEMENTED — a layer-6 static grants "can't attack" to every creature
// without flying.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOAT,
    oracle_id = "42208fea-8c24-451f-861d-6d70c0a7a502",
    scryfall_id = "e2dffeb3-c858-4b8c-ae1f-109721f7d2da",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Moat",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING))
        ]),
        Modifier::AddKeyword(KeywordSet::CANT_ATTACK)
    )],
);
