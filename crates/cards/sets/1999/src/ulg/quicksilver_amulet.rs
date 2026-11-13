//! Quicksilver Amulet — {4} — Artifact
//! Oracle: {4}, {T}: You may put a creature card from your hand onto the battlefield.
//! Set: M12 #214 — Magic 2012 | Scryfall ID: 04c0357a-e98d-4c49-83ad-d7a8ebe7e2d1 | Oracle ID: e1096a98-a631-43f5-97d8-325001d608b3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::QUICKSILVER_AMULET,
    oracle_id = "e1096a98-a631-43f5-97d8-325001d608b3",
    scryfall_id = "04c0357a-e98d-4c49-83ad-d7a8ebe7e2d1",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Quicksilver Amulet",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::CREATURE,
            mana_value: None,
            optional: true,
        }],
    )],
);
