//! Terrain Generator — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You may put a basic land card from your hand onto the battlefield tapped.
//! Set: CLB #926 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: f40e9f0f-8c0d-4bfd-9872-370ce3763006 | Oracle ID: a949c96c-362c-45a3-bd5c-ce5ce153ee9e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TERRAIN_GENERATOR,
    oracle_id = "a949c96c-362c-45a3-bd5c-ce5ce153ee9e",
    scryfall_id = "f40e9f0f-8c0d-4bfd-9872-370ce3763006",
    faces = &[face!(name = "Terrain Generator", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::PutFromHandOntoBattlefield {
                tapped: true,
                filter: &Filter::BASIC_LAND,
                mana_value: None,
                optional: true
            }]
        ),
    ],
);
