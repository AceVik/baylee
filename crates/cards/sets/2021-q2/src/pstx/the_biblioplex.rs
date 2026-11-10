//! The Biblioplex — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Look at the top card of your library. If it's an instant or sorcery card, you may reveal it and put it into your hand. If you don't put the card into your hand, you may put it into your graveyard. Activate only if you have exactly zero or seven cards in hand.
//! Set: STX #264 — Strixhaven: School of Mages | Scryfall ID: 8eae4481-977f-4bb7-bb1a-ab7100e6ba47 | Oracle ID: 86ed6073-c35c-4d29-9911-5fe191dd875f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_BIBLIOPLEX,
    oracle_id = "86ed6073-c35c-4d29-9911-5fe191dd875f",
    scryfall_id = "8eae4481-977f-4bb7-bb1a-ab7100e6ba47",
    faces = &[face!(name = "The Biblioplex", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::LookAtTopMayTake {
                filter: &Filter::INSTANT_OR_SORCERY,
                rest: LookRest::MayGraveyard,
                then: &[],
            }],
            condition = Some(Condition::Any(&[
                Condition::HandSizeExactly(0),
                Condition::HandSizeExactly(7)
            ]))
        ),
    ],
);
