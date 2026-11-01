//! Call of the Wild — {2}{G}{G} — Enchantment
//! Oracle: {2}{G}{G}: Reveal the top card of your library. If it's a creature card, put it onto the battlefield. Otherwise, put it into your graveyard.
//! Set: 8ED #235 — Eighth Edition | Scryfall ID: 3332b35c-fd4f-4195-baee-e759dee6a682 | Oracle ID: 882ee72e-2149-4778-a81c-8bf09beb5d66
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CALL_OF_THE_WILD,
    oracle_id = "882ee72e-2149-4778-a81c-8bf09beb5d66",
    scryfall_id = "3332b35c-fd4f-4195-baee-e759dee6a682",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Call of the Wild",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{G}{G}"),
        &[Effect::RevealTopAndSort {
            filter: &Filter::CREATURE,
            matched: SearchDest::Battlefield,
            otherwise: SearchDest::Graveyard,
        }],
    )],
);
