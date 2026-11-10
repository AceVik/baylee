//! Night Soil — {G}{G} — Enchantment
//! Oracle: {1}, Exile two creature cards from a single graveyard: Create a 1/1 green Saproling creature token.
//! Set: C13 #158 — Commander 2013 | Scryfall ID: 52a0eca1-f936-4f5a-820b-fa12542c593d | Oracle ID: 3165fe8f-52d7-40f7-bb14-8f4300a564e6
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::NIGHT_SOIL,
    oracle_id = "3165fe8f-52d7-40f7-bb14-8f4300a564e6",
    scryfall_id = "52a0eca1-f936-4f5a-820b-fa12542c593d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Night Soil",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(
            "{1}",
            ExileFromSingleGraveyard {
                filter: &Filter::CREATURE,
                count: 2,
            }
        ),
        &[Effect::CreateToken {
            token: &generated_tokens::SAPROLING_1_1_GREEN,
        }],
    ),],
);
