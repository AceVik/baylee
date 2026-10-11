//! War Room — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card.
//! Set: SOC #422 — Secrets of Strixhaven Commander | Scryfall ID: 0775b4be-881c-4832-8954-d961064315b6 | Oracle ID: 71c52bf5-2a5d-488e-8b15-7ef290e4b77d
use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_ROOM,
    oracle_id = "71c52bf5-2a5d-488e-8b15-7ef290e4b77d",
    scryfall_id = "0775b4be-881c-4832-8954-d961064315b6",
    faces = &[face!(name = "War Room", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf, PayCommanderColors),
            &[Effect::DrawCards {
                amount: Amount::Fixed(1)
            }]
        ),
    ],
);
