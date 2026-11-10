//! Geier Reach Sanitarium — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Each player draws a card, then discards a card.
//! Set: LCC #335 — The Lost Caverns of Ixalan Commander | Scryfall ID: 4b9c92f0-4242-4a3e-9ede-6a4935f5c75d | Oracle ID: 7b9fafe7-d26a-4ed5-b4c4-ce13763770b5
// IMPLEMENTED — {T}: Add {C}; {2}, {T}: every player draws a card, then every
// player discards a card (DrawCardsFor + DiscardForPlayers over EachPlayer).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GEIER_REACH_SANITARIUM,
    oracle_id = "7b9fafe7-d26a-4ed5-b4c4-ce13763770b5",
    scryfall_id = "4b9c92f0-4242-4a3e-9ede-6a4935f5c75d",
    faces = &[face!(
        name = "Geier Reach Sanitarium",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::EachPlayer,
                },
                Effect::DiscardForPlayers {
                    who: PlayerRel::EachPlayer,
                    count: 1,
                },
            ]
        ),
    ],
);
