//! The Biblioplex — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Look at the top card of your library. If it's an instant or sorcery card, you may reveal it and put it into your hand. If you don't put the card into your hand, you may put it into your graveyard. Activate only if you have exactly zero or seven cards in hand.
//! Set: STX #264 — Strixhaven: School of Mages | Scryfall ID: 8eae4481-977f-4bb7-bb1a-ab7100e6ba47 | Oracle ID: 86ed6073-c35c-4d29-9911-5fe191dd875f
// PARTIAL — mana ability built; the {2}, {T} ability is dropped, see below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_BIBLIOPLEX,
    oracle_id = "86ed6073-c35c-4d29-9911-5fe191dd875f",
    scryfall_id = "8eae4481-977f-4bb7-bb1a-ab7100e6ba47",
    faces = &[face!(name = "The Biblioplex", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {2}, {T} ability is dropped: no Condition reads a player's hand \
         size, and no Effect looks at the top card and sends it to hand or \
         graveyard",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}: Look at the top card of your library. If it's an instant or sorcery card, you may reveal it and put it into your hand. If you don't put the card into your hand, you may put it into your graveyard. Activate only if you have exactly zero or seven cards in hand."
    ],
);
