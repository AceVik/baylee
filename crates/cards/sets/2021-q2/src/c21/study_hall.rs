//! Study Hall — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color. When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game.
//! Set: SOC #407 — Secrets of Strixhaven Commander | Scryfall ID: 93f812bc-7663-4504-b566-79e2c0a0f845 | Oracle ID: eb735501-19e7-4910-aa6a-6667fff6f4e5

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUDY_HALL,
    oracle_id = "eb735501-19e7-4910-aa6a-6667fff6f4e5",
    scryfall_id = "93f812bc-7663-4504-b566-79e2c0a0f845",
    faces = &[face!(name = "Study Hall", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{1}", TapSelf),
            &[Effect::mana_of_any_color().when_spent(
                &Filter::And(&[Filter::IsCommander, Filter::OwnedByYou]),
                SpendRider::CommanderScry
            )]
        ),
    ],
);
