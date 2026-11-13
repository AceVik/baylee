//! Mana Breach — {2}{U} — Enchantment
//! Oracle: Whenever a player casts a spell, that player returns a land they control to its owner's hand.
//! Set: 7ED #85 — Seventh Edition | Scryfall ID: e36a7eff-89c8-4799-b264-38892912ba05 | Oracle ID: 221d0305-94db-4791-9dc7-89ee96c7fcd6
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_BREACH,
    oracle_id = "221d0305-94db-4791-9dc7-89ee96c7fcd6",
    scryfall_id = "e36a7eff-89c8-4799-b264-38892912ba05",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mana Breach",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::Any),
        &[Effect::ReturnChosenToHand {
            who: PlayerRel::EventPlayer,
            filter: &Filter::LAND,
        }],
    )],
);
