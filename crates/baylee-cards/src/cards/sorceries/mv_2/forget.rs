//! Forget — {U}{U} — Sorcery
//! Oracle: Target player discards two cards, then draws as many cards as they discarded this way.
//! Set: 6ED #70 — Classic Sixth Edition | Scryfall ID: 8cc8e367-1aa4-43b6-b17a-01bfb097f620 | Oracle ID: 619ef7e1-33cd-4470-a1d4-83c5f1f5c31e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGET,
    oracle_id = "619ef7e1-33cd-4470-a1d4-83c5f1f5c31e",
    scryfall_id = "8cc8e367-1aa4-43b6-b17a-01bfb097f620",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Forget",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DiscardForPlayers {
                who: PlayerRel::Chosen,
                count: 2,
            },
            Effect::DrawCardsFor {
                amount: Amount::ThisWay(ThisWay::Discarded),
                who: PlayerRel::Chosen,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
